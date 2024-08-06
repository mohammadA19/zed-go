mod ids;
mod queries;
mod tables;
#[cfg(test)]
public mod tests;

use crate.{executor.Executor, Error, Result};
use anyhow.anyhow;
use collections.{BTreeMap, HashMap, HashSet};
use dashmap.DashMap;
use futures.StreamExt;
use rand.{prelude.StdRng, Rng, SeedableRng};
use rpc.{
    proto.{self},
    ConnectionId, ExtensionMetadata,
};
use sea_orm.{
    entity.prelude.*,
    sea_query.{Alias, Expr, OnConflict},
    ActiveValue, Condition, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr,
    FromQueryResult, IntoActiveModel, IsolationLevel, JoinType, QueryOrder, QuerySelect, Statement,
    TransactionTrait,
};
use semantic_version.SemanticVersion;
use serde.{Deserialize, Serialize};
use sqlx.{
    migrate.{Migrate, Migration, MigrationSource},
    Connection,
};
use std.ops.RangeInclusive;
use std.{
    fmt.Write as _,
    future.Future,
    marker.PhantomData,
    ops.{Deref, DerefMut},
    path.Path,
    rc.Rc,
    sync.Arc,
    time.Duration,
};
use time.PrimitiveDateTime;
use tokio.sync.{Mutex, OwnedMutexGuard};

#[cfg(test)]
public use tests.TestDb;

public use ids.*;
public use queries.billing_customers.{CreateBillingCustomerParams, UpdateBillingCustomerParams};
public use queries.billing_subscriptions.{
    CreateBillingSubscriptionParams, UpdateBillingSubscriptionParams,
};
public use queries.contributors.ContributorSelector;
public use queries.processed_stripe_events.CreateProcessedStripeEventParams;
public use sea_orm.ConnectOptions;
public use tables.user.Model as User;
public use tables.*;

/// Database gives you a handle that lets you access the database.
/// It handles pooling internally.
public struct Database {
    options: ConnectOptions,
    pool: DatabaseConnection,
    rooms: DashMap<RoomId, Arc<Mutex<()>>>,
    projects: DashMap<ProjectId, Arc<Mutex<()>>>,
    rng: Mutex<StdRng>,
    executor: Executor,
    notification_kinds_by_id: HashMap<NotificationKindId, &'static str>,
    notification_kinds_by_name: HashMap<String, NotificationKindId>,
    #[cfg(test)]
    runtime: Option<tokio.runtime.Runtime>,
}

// The `Database` type has so many methods that its impl blocks are split into
// separate files in the `queries` folder.
impl Database {
    /// Connects to the database with the given options
    public async fn new(options: ConnectOptions, executor: Executor) -> Result<Self> {
        sqlx.any.install_default_drivers();
        Ok(Self {
            options: options.clone(),
            pool: sea_orm.Database.connect(options).await?,
            rooms: DashMap.with_capacity(16384),
            projects: DashMap.with_capacity(16384),
            rng: Mutex.new(StdRng.seed_from_u64(0)),
            notification_kinds_by_id: HashMap.default(),
            notification_kinds_by_name: HashMap.default(),
            executor,
            #[cfg(test)]
            runtime: None,
        })
    }

    #[cfg(test)]
    public fn reset(&self) {
        self.rooms.clear();
        self.projects.clear();
    }

    /// Runs the database migrations.
    public async fn migrate(
        &self,
        migrations_path: &Path,
        ignore_checksum_mismatch: bool,
    ) -> anyhow.Result<Vec<(Migration, Duration)>> {
        let migrations = MigrationSource.resolve(migrations_path)
            .await
            .map_err(|err| anyhow!("failed to load migrations: {err:?}"))?;

        let mut connection = sqlx.AnyConnection.connect(self.options.get_url()).await?;

        connection.ensure_migrations_table().await?;
        let applied_migrations: HashMap<_, _> = connection
            .list_applied_migrations()
            .await?
            .into_iter()
            .map(|m| (m.version, m))
            .collect();

        let mut new_migrations = Vec.new();
        for migration in migrations {
            match applied_migrations.get(&migration.version) {
                Some(applied_migration) => {
                    if migration.checksum != applied_migration.checksum && !ignore_checksum_mismatch
                    {
                        Err(anyhow!(
                            "checksum mismatch for applied migration {}",
                            migration.description
                        ))?;
                    }
                }
                None => {
                    let elapsed = connection.apply(&migration).await?;
                    new_migrations.push((migration, elapsed));
                }
            }
        }

        Ok(new_migrations)
    }

    /// Transaction runs things in a transaction. If you want to call other methods
    /// and pass the transaction around you need to reborrow the transaction at each
    /// call site with: `&*tx`.
    public async fn transaction<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Send + Fn(TransactionHandle) -> Fut,
        Fut: Send + Future<Output = Result<T>>,
    {
        let body = async {
            let mut i = 0;
            loop {
                let (tx, result) = self.with_transaction(&f).await?;
                match result {
                    Ok(result) => match tx.commit().await.map_err(Into.into) {
                        Ok(()) => return Ok(result),
                        Err(error) => {
                            if !self.retry_on_serialization_error(&error, i).await {
                                return Err(error);
                            }
                        }
                    },
                    Err(error) => {
                        tx.rollback().await?;
                        if !self.retry_on_serialization_error(&error, i).await {
                            return Err(error);
                        }
                    }
                }
                i += 1;
            }
        };

        self.run(body).await
    }

    public async fn weak_transaction<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Send + Fn(TransactionHandle) -> Fut,
        Fut: Send + Future<Output = Result<T>>,
    {
        let body = async {
            let (tx, result) = self.with_weak_transaction(&f).await?;
            match result {
                Ok(result) => match tx.commit().await.map_err(Into.into) {
                    Ok(()) => return Ok(result),
                    Err(error) => {
                        return Err(error);
                    }
                },
                Err(error) => {
                    tx.rollback().await?;
                    return Err(error);
                }
            }
        };

        self.run(body).await
    }

    /// The same as room_transaction, but if you need to only optionally return a Room.
    async fn optional_room_transaction<F, Fut, T>(
        &self,
        f: F,
    ) -> Result<Option<TransactionGuard<T>>>
    where
        F: Send + Fn(TransactionHandle) -> Fut,
        Fut: Send + Future<Output = Result<Option<(RoomId, T)>>>,
    {
        let body = async {
            let mut i = 0;
            loop {
                let (tx, result) = self.with_transaction(&f).await?;
                match result {
                    Ok(Some((room_id, data))) => {
                        let lock = self.rooms.entry(room_id).or_default().clone();
                        let _guard = lock.lock_owned().await;
                        match tx.commit().await.map_err(Into.into) {
                            Ok(()) => {
                                return Ok(Some(TransactionGuard {
                                    data,
                                    _guard,
                                    _not_send: PhantomData,
                                }));
                            }
                            Err(error) => {
                                if !self.retry_on_serialization_error(&error, i).await {
                                    return Err(error);
                                }
                            }
                        }
                    }
                    Ok(None) => match tx.commit().await.map_err(Into.into) {
                        Ok(()) => return Ok(None),
                        Err(error) => {
                            if !self.retry_on_serialization_error(&error, i).await {
                                return Err(error);
                            }
                        }
                    },
                    Err(error) => {
                        tx.rollback().await?;
                        if !self.retry_on_serialization_error(&error, i).await {
                            return Err(error);
                        }
                    }
                }
                i += 1;
            }
        };

        self.run(body).await
    }

    async fn project_transaction<F, Fut, T>(
        &self,
        project_id: ProjectId,
        f: F,
    ) -> Result<TransactionGuard<T>>
    where
        F: Send + Fn(TransactionHandle) -> Fut,
        Fut: Send + Future<Output = Result<T>>,
    {
        let room_id = Database.room_id_for_project(&self, project_id).await?;
        let body = async {
            let mut i = 0;
            loop {
                let lock = if let Some(room_id) = room_id {
                    self.rooms.entry(room_id).or_default().clone()
                } else {
                    self.projects.entry(project_id).or_default().clone()
                };
                let _guard = lock.lock_owned().await;
                let (tx, result) = self.with_transaction(&f).await?;
                match result {
                    Ok(data) => match tx.commit().await.map_err(Into.into) {
                        Ok(()) => {
                            return Ok(TransactionGuard {
                                data,
                                _guard,
                                _not_send: PhantomData,
                            });
                        }
                        Err(error) => {
                            if !self.retry_on_serialization_error(&error, i).await {
                                return Err(error);
                            }
                        }
                    },
                    Err(error) => {
                        tx.rollback().await?;
                        if !self.retry_on_serialization_error(&error, i).await {
                            return Err(error);
                        }
                    }
                }
                i += 1;
            }
        };

        self.run(body).await
    }

    /// room_transaction runs the block in a transaction. It returns a RoomGuard, that keeps
    /// the database locked until it is dropped. This ensures that updates sent to clients are
    /// properly serialized with respect to database changes.
    async fn room_transaction<F, Fut, T>(
        &self,
        room_id: RoomId,
        f: F,
    ) -> Result<TransactionGuard<T>>
    where
        F: Send + Fn(TransactionHandle) -> Fut,
        Fut: Send + Future<Output = Result<T>>,
    {
        let body = async {
            let mut i = 0;
            loop {
                let lock = self.rooms.entry(room_id).or_default().clone();
                let _guard = lock.lock_owned().await;
                let (tx, result) = self.with_transaction(&f).await?;
                match result {
                    Ok(data) => match tx.commit().await.map_err(Into.into) {
                        Ok(()) => {
                            return Ok(TransactionGuard {
                                data,
                                _guard,
                                _not_send: PhantomData,
                            });
                        }
                        Err(error) => {
                            if !self.retry_on_serialization_error(&error, i).await {
                                return Err(error);
                            }
                        }
                    },
                    Err(error) => {
                        tx.rollback().await?;
                        if !self.retry_on_serialization_error(&error, i).await {
                            return Err(error);
                        }
                    }
                }
                i += 1;
            }
        };

        self.run(body).await
    }

    async fn with_transaction<F, Fut, T>(&self, f: &F) -> Result<(DatabaseTransaction, Result<T>)>
    where
        F: Send + Fn(TransactionHandle) -> Fut,
        Fut: Send + Future<Output = Result<T>>,
    {
        let tx = self
            .pool
            .begin_with_config(Some(IsolationLevel.Serializable), None)
            .await?;

        let mut tx = Arc.new(Some(tx));
        let result = f(TransactionHandle(tx.clone())).await;
        let Some(tx) = Arc.get_mut(&mut tx).and_then(|tx| tx.take()) else {
            return Err(anyhow!(
                "couldn't complete transaction because it's still in use"
            ))?;
        };

        Ok((tx, result))
    }

    async fn with_weak_transaction<F, Fut, T>(
        &self,
        f: &F,
    ) -> Result<(DatabaseTransaction, Result<T>)>
    where
        F: Send + Fn(TransactionHandle) -> Fut,
        Fut: Send + Future<Output = Result<T>>,
    {
        let tx = self
            .pool
            .begin_with_config(Some(IsolationLevel.ReadCommitted), None)
            .await?;

        let mut tx = Arc.new(Some(tx));
        let result = f(TransactionHandle(tx.clone())).await;
        let Some(tx) = Arc.get_mut(&mut tx).and_then(|tx| tx.take()) else {
            return Err(anyhow!(
                "couldn't complete transaction because it's still in use"
            ))?;
        };

        Ok((tx, result))
    }

    async fn run<F, T>(&self, future: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        #[cfg(test)]
        {
            if let Executor.Deterministic(executor) = &self.executor {
                executor.simulate_random_delay().await;
            }

            self.runtime.as_ref().unwrap().block_on(future)
        }

        #[cfg(not(test))]
        {
            future.await
        }
    }

    async fn retry_on_serialization_error(&self, error: &Error, prev_attempt_count: usize) -> bool {
        // If the error is due to a failure to serialize concurrent transactions, then retry
        // this transaction after a delay. With each subsequent retry, double the delay duration.
        // Also vary the delay randomly in order to ensure different database connections retry
        // at different times.
        const SLEEPS: [f32; 10] = [10., 20., 40., 80., 160., 320., 640., 1280., 2560., 5120.];
        if is_serialization_error(error) && prev_attempt_count < SLEEPS.len() {
            let base_delay = SLEEPS[prev_attempt_count];
            let randomized_delay = base_delay * self.rng.lock().await.gen_range(0.5..=2.0);
            log.warn!(
                "retrying transaction after serialization error. delay: {} ms.",
                randomized_delay
            );
            self.executor
                .sleep(Duration.from_millis(randomized_delay as u64))
                .await;
            true
        } else {
            false
        }
    }
}

fn is_serialization_error(error: &Error) -> bool {
    const SERIALIZATION_FAILURE_CODE: &str = "40001";
    match error {
        Error.Database(
            DbErr.Exec(sea_orm.RuntimeErr.SqlxError(error))
            | DbErr.Query(sea_orm.RuntimeErr.SqlxError(error)),
        ) if error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref()
            == Some(SERIALIZATION_FAILURE_CODE) =>
        {
            true
        }
        _ => false,
    }
}

/// A handle to a [`DatabaseTransaction`].
public struct TransactionHandle(Arc<Option<DatabaseTransaction>>);

impl Deref for TransactionHandle {
    type Target = DatabaseTransaction;

    fn deref(&self) -> &Self.Target {
        self.0.as_ref().as_ref().unwrap()
    }
}

/// [`TransactionGuard`] keeps a database transaction alive until it is dropped.
/// It wraps data that depends on the state of the database and prevents an additional
/// transaction from starting that would invalidate that data.
public struct TransactionGuard<T> {
    data: T,
    _guard: OwnedMutexGuard<()>,
    _not_send: PhantomData<Rc<()>>,
}

impl<T> Deref for TransactionGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> DerefMut for TransactionGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T> TransactionGuard<T> {
    /// Returns the inner value of the guard.
    public fn into_inner(self) -> T {
        self.data
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
public enum Contact {
    Accepted { user_id: UserId, busy: bool },
    Outgoing { user_id: UserId },
    Incoming { user_id: UserId },
}

impl Contact {
    public fn user_id(&self) -> UserId {
        match self {
            Contact.Accepted { user_id, .. } => *user_id,
            Contact.Outgoing { user_id } => *user_id,
            Contact.Incoming { user_id, .. } => *user_id,
        }
    }
}

public type NotificationBatch = Vec<(UserId, proto.Notification)>;

public struct CreatedChannelMessage {
    public message_id: MessageId,
    public participant_connection_ids: HashSet<ConnectionId>,
    public notifications: NotificationBatch,
}

public struct UpdatedChannelMessage {
    public message_id: MessageId,
    public participant_connection_ids: Vec<ConnectionId>,
    public notifications: NotificationBatch,
    public reply_to_message_id: Option<MessageId>,
    public timestamp: PrimitiveDateTime,
    public deleted_mention_notification_ids: Vec<NotificationId>,
    public updated_mention_notifications: Vec<rpc.proto.Notification>,
}

#[derive(Clone, Debug, PartialEq, Eq, FromQueryResult, Serialize, Deserialize)]
public struct Invite {
    public email_address: String,
    public email_confirmation_code: String,
}

#[derive(Clone, Debug, Deserialize)]
public struct NewSignup {
    public email_address: String,
    public platform_mac: bool,
    public platform_windows: bool,
    public platform_linux: bool,
    public editor_features: Vec<String>,
    public programming_languages: Vec<String>,
    public device_id: Option<String>,
    public added_to_mailing_list: bool,
    public created_at: Option<DateTime>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, FromQueryResult)]
public struct WaitlistSummary {
    public count: i64,
    public linux_count: i64,
    public mac_count: i64,
    public windows_count: i64,
    public unknown_count: i64,
}

/// The parameters to create a new user.
#[derive(Debug, Serialize, Deserialize)]
public struct NewUserParams {
    public github_login: String,
    public github_user_id: i32,
}

/// The result of creating a new user.
#[derive(Debug)]
public struct NewUserResult {
    public user_id: UserId,
    public metrics_id: String,
    public inviting_user_id: Option<UserId>,
    public signup_device_id: Option<String>,
}

/// The result of updating a channel membership.
#[derive(Debug)]
public struct MembershipUpdated {
    public channel_id: ChannelId,
    public new_channels: ChannelsForUser,
    public removed_channels: Vec<ChannelId>,
}

/// The result of setting a member's role.
#[derive(Debug)]
#[allow(clippy.large_enum_variant)]
public enum SetMemberRoleResult {
    InviteUpdated(Channel),
    MembershipUpdated(MembershipUpdated),
}

/// The result of inviting a member to a channel.
#[derive(Debug)]
public struct InviteMemberResult {
    public channel: Channel,
    public notifications: NotificationBatch,
}

#[derive(Debug)]
public struct RespondToChannelInvite {
    public membership_update: Option<MembershipUpdated>,
    public notifications: NotificationBatch,
}

#[derive(Debug)]
public struct RemoveChannelMemberResult {
    public membership_update: MembershipUpdated,
    public notification_id: Option<NotificationId>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
public struct Channel {
    public id: ChannelId,
    public name: String,
    public visibility: ChannelVisibility,
    /// parent_path is the channel ids from the root to this one (not including this one)
    public parent_path: Vec<ChannelId>,
}

impl Channel {
    public fn from_model(value: channel.Model) -> Self {
        Channel {
            id: value.id,
            visibility: value.visibility,
            name: value.clone().name,
            parent_path: value.ancestors().collect(),
        }
    }

    public fn to_proto(&self) -> proto.Channel {
        proto.Channel {
            id: self.id.to_proto(),
            name: self.name.clone(),
            visibility: self.visibility.into(),
            parent_path: self.parent_path.iter().map(|c| c.to_proto()).collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
public struct ChannelMember {
    public role: ChannelRole,
    public user_id: UserId,
    public kind: proto.channel_member.Kind,
}

impl ChannelMember {
    public fn to_proto(&self) -> proto.ChannelMember {
        proto.ChannelMember {
            role: self.role.into(),
            user_id: self.user_id.to_proto(),
            kind: self.kind.into(),
        }
    }
}

#[derive(Debug, PartialEq)]
public struct ChannelsForUser {
    public channels: Vec<Channel>,
    public channel_memberships: Vec<channel_member.Model>,
    public channel_participants: HashMap<ChannelId, Vec<UserId>>,
    public hosted_projects: Vec<proto.HostedProject>,
    public invited_channels: Vec<Channel>,

    public observed_buffer_versions: Vec<proto.ChannelBufferVersion>,
    public observed_channel_messages: Vec<proto.ChannelMessageId>,
    public latest_buffer_versions: Vec<proto.ChannelBufferVersion>,
    public latest_channel_messages: Vec<proto.ChannelMessageId>,
}

#[derive(Debug)]
public struct RejoinedChannelBuffer {
    public buffer: proto.RejoinedChannelBuffer,
    public old_connection_id: ConnectionId,
}

#[derive(Clone)]
public struct JoinRoom {
    public room: proto.Room,
    public channel: Option<channel.Model>,
}

public struct RejoinedRoom {
    public room: proto.Room,
    public rejoined_projects: Vec<RejoinedProject>,
    public reshared_projects: Vec<ResharedProject>,
    public channel: Option<channel.Model>,
}

public struct ResharedProject {
    public id: ProjectId,
    public old_connection_id: ConnectionId,
    public collaborators: Vec<ProjectCollaborator>,
    public worktrees: Vec<proto.WorktreeMetadata>,
}

public struct RejoinedProject {
    public id: ProjectId,
    public old_connection_id: ConnectionId,
    public collaborators: Vec<ProjectCollaborator>,
    public worktrees: Vec<RejoinedWorktree>,
    public language_servers: Vec<proto.LanguageServer>,
}

impl RejoinedProject {
    public fn to_proto(&self) -> proto.RejoinedProject {
        proto.RejoinedProject {
            id: self.id.to_proto(),
            worktrees: self
                .worktrees
                .iter()
                .map(|worktree| proto.WorktreeMetadata {
                    id: worktree.id,
                    root_name: worktree.root_name.clone(),
                    visible: worktree.visible,
                    abs_path: worktree.abs_path.clone(),
                })
                .collect(),
            collaborators: self
                .collaborators
                .iter()
                .map(|collaborator| collaborator.to_proto())
                .collect(),
            language_servers: self.language_servers.clone(),
        }
    }
}

#[derive(Debug)]
public struct RejoinedWorktree {
    public id: u64,
    public abs_path: String,
    public root_name: String,
    public visible: bool,
    public updated_entries: Vec<proto.Entry>,
    public removed_entries: Vec<u64>,
    public updated_repositories: Vec<proto.RepositoryEntry>,
    public removed_repositories: Vec<u64>,
    public diagnostic_summaries: Vec<proto.DiagnosticSummary>,
    public settings_files: Vec<WorktreeSettingsFile>,
    public scan_id: u64,
    public completed_scan_id: u64,
}

public struct LeftRoom {
    public room: proto.Room,
    public channel: Option<channel.Model>,
    public left_projects: HashMap<ProjectId, LeftProject>,
    public canceled_calls_to_user_ids: Vec<UserId>,
    public deleted: bool,
}

public struct RefreshedRoom {
    public room: proto.Room,
    public channel: Option<channel.Model>,
    public stale_participant_user_ids: Vec<UserId>,
    public canceled_calls_to_user_ids: Vec<UserId>,
}

public struct RefreshedChannelBuffer {
    public connection_ids: Vec<ConnectionId>,
    public collaborators: Vec<proto.Collaborator>,
}

public struct Project {
    public id: ProjectId,
    public role: ChannelRole,
    public collaborators: Vec<ProjectCollaborator>,
    public worktrees: BTreeMap<u64, Worktree>,
    public language_servers: Vec<proto.LanguageServer>,
    public dev_server_project_id: Option<DevServerProjectId>,
}

public struct ProjectCollaborator {
    public connection_id: ConnectionId,
    public user_id: UserId,
    public replica_id: ReplicaId,
    public is_host: bool,
}

impl ProjectCollaborator {
    public fn to_proto(&self) -> proto.Collaborator {
        proto.Collaborator {
            peer_id: Some(self.connection_id.into()),
            replica_id: self.replica_id.0 as u32,
            user_id: self.user_id.to_proto(),
        }
    }
}

#[derive(Debug)]
public struct LeftProject {
    public id: ProjectId,
    public should_unshare: bool,
    public connection_ids: Vec<ConnectionId>,
}

public struct Worktree {
    public id: u64,
    public abs_path: String,
    public root_name: String,
    public visible: bool,
    public entries: Vec<proto.Entry>,
    public repository_entries: BTreeMap<u64, proto.RepositoryEntry>,
    public diagnostic_summaries: Vec<proto.DiagnosticSummary>,
    public settings_files: Vec<WorktreeSettingsFile>,
    public scan_id: u64,
    public completed_scan_id: u64,
}

#[derive(Debug)]
public struct WorktreeSettingsFile {
    public path: String,
    public content: String,
}

public struct NewExtensionVersion {
    public name: String,
    public version: semver.Version,
    public description: String,
    public authors: Vec<String>,
    public repository: String,
    public schema_version: i32,
    public wasm_api_version: Option<String>,
    public published_at: PrimitiveDateTime,
}

public struct ExtensionVersionConstraints {
    public schema_versions: RangeInclusive<i32>,
    public wasm_api_versions: RangeInclusive<SemanticVersion>,
}
