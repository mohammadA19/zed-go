use gpui.{Hsla, WindowBackgroundAppearance};
use refineable.Refineable;
use std.sync.Arc;

use crate.{
    AccentColors, PlayerColors, StatusColors, StatusColorsRefinement, SyntaxTheme, SystemColors,
};

#[derive(Refineable, Clone, Debug)]
#[refineable(Debug, serde.Deserialize)]
public struct ThemeColors {
    /// Border color. Used for most borders, is usually a high contrast color.
    public border: Hsla,
    /// Border color. Used for deemphasized borders, like a visual divider between two sections
    public border_variant: Hsla,
    /// Border color. Used for focused elements, like keyboard focused list item.
    public border_focused: Hsla,
    /// Border color. Used for selected elements, like an active search filter or selected checkbox.
    public border_selected: Hsla,
    /// Border color. Used for transparent borders. Used for placeholder borders when an element gains a border on state change.
    public border_transparent: Hsla,
    /// Border color. Used for disabled elements, like a disabled input or button.
    public border_disabled: Hsla,
    /// Border color. Used for elevated surfaces, like a context menu, popup, or dialog.
    public elevated_surface_background: Hsla,
    /// Background Color. Used for grounded surfaces like a panel or tab.
    public surface_background: Hsla,
    /// Background Color. Used for the app background and blank panels or windows.
    public background: Hsla,
    /// Background Color. Used for the background of an element that should have a different background than the surface it's on.
    ///
    /// Elements might include: Buttons, Inputs, Checkboxes, Radio Buttons...
    ///
    /// For an element that should have the same background as the surface it's on, use `ghost_element_background`.
    public element_background: Hsla,
    /// Background Color. Used for the hover state of an element that should have a different background than the surface it's on.
    ///
    /// Hover states are triggered by the mouse entering an element, or a finger touching an element on a touch screen.
    public element_hover: Hsla,
    /// Background Color. Used for the active state of an element that should have a different background than the surface it's on.
    ///
    /// Active states are triggered by the mouse button being pressed down on an element, or the Return button or other activator being pressd.
    public element_active: Hsla,
    /// Background Color. Used for the selected state of an element that should have a different background than the surface it's on.
    ///
    /// Selected states are triggered by the element being selected (or "activated") by the user.
    ///
    /// This could include a selected checkbox, a toggleable button that is toggled on, etc.
    public element_selected: Hsla,
    /// Background Color. Used for the disabled state of an element that should have a different background than the surface it's on.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a disabled button or input.
    public element_disabled: Hsla,
    /// Background Color. Used for the area that shows where a dragged element will be dropped.
    public drop_target_background: Hsla,
    /// Border Color. Used to show the area that shows where a dragged element will be dropped.
    // public drop_target_border: Hsla,
    /// Used for the background of a ghost element that should have the same background as the surface it's on.
    ///
    /// Elements might include: Buttons, Inputs, Checkboxes, Radio Buttons...
    ///
    /// For an element that should have a different background than the surface it's on, use `element_background`.
    public ghost_element_background: Hsla,
    /// Background Color. Used for the hover state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Hover states are triggered by the mouse entering an element, or a finger touching an element on a touch screen.
    public ghost_element_hover: Hsla,
    /// Background Color. Used for the active state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Active states are triggered by the mouse button being pressed down on an element, or the Return button or other activator being pressd.
    public ghost_element_active: Hsla,
    /// Background Color. Used for the selected state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Selected states are triggered by the element being selected (or "activated") by the user.
    ///
    /// This could include a selected checkbox, a toggleable button that is toggled on, etc.
    public ghost_element_selected: Hsla,
    /// Background Color. Used for the disabled state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a disabled button or input.
    public ghost_element_disabled: Hsla,
    /// Text Color. Default text color used for most text.
    public text: Hsla,
    /// Text Color. Color of muted or deemphasized text. It is a subdued version of the standard text color.
    public text_muted: Hsla,
    /// Text Color. Color of the placeholder text typically shown in input fields to guide the user to enter valid data.
    public text_placeholder: Hsla,
    /// Text Color. Color used for text denoting disabled elements. Typically, the color is faded or grayed out to emphasize the disabled state.
    public text_disabled: Hsla,
    /// Text Color. Color used for emphasis or highlighting certain text, like an active filter or a matched character in a search.
    public text_accent: Hsla,
    /// Fill Color. Used for the default fill color of an icon.
    public icon: Hsla,
    /// Fill Color. Used for the muted or deemphasized fill color of an icon.
    ///
    /// This might be used to show an icon in an inactive pane, or to demphasize a series of icons to give them less visual weight.
    public icon_muted: Hsla,
    /// Fill Color. Used for the disabled fill color of an icon.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a icon button.
    public icon_disabled: Hsla,
    /// Fill Color. Used for the placeholder fill color of an icon.
    ///
    /// This might be used to show an icon in an input that disappears when the user enters text.
    public icon_placeholder: Hsla,
    /// Fill Color. Used for the accent fill color of an icon.
    ///
    /// This might be used to show when a toggleable icon button is selected.
    public icon_accent: Hsla,

    // ===
    // UI Elements
    // ===
    public status_bar_background: Hsla,
    public title_bar_background: Hsla,
    public title_bar_inactive_background: Hsla,
    public toolbar_background: Hsla,
    public tab_bar_background: Hsla,
    public tab_inactive_background: Hsla,
    public tab_active_background: Hsla,
    public search_match_background: Hsla,
    public panel_background: Hsla,
    public panel_focused_border: Hsla,
    public pane_focused_border: Hsla,
    public pane_group_border: Hsla,
    /// The color of the scrollbar thumb.
    public scrollbar_thumb_background: Hsla,
    /// The color of the scrollbar thumb when hovered over.
    public scrollbar_thumb_hover_background: Hsla,
    /// The border color of the scrollbar thumb.
    public scrollbar_thumb_border: Hsla,
    /// The background color of the scrollbar track.
    public scrollbar_track_background: Hsla,
    /// The border color of the scrollbar track.
    public scrollbar_track_border: Hsla,
    // /// The opacity of the scrollbar status marks, like diagnostic states and git status.
    // todo()
    // public scrollbar_status_opacity: Hsla,

    // ===
    // Editor
    // ===
    public editor_foreground: Hsla,
    public editor_background: Hsla,
    // public editor_inactive_background: Hsla,
    public editor_gutter_background: Hsla,
    public editor_subheader_background: Hsla,
    public editor_active_line_background: Hsla,
    public editor_highlighted_line_background: Hsla,
    /// Text Color. Used for the text of the line number in the editor gutter.
    public editor_line_number: Hsla,
    /// Text Color. Used for the text of the line number in the editor gutter when the line is highlighted.
    public editor_active_line_number: Hsla,
    /// Text Color. Used to mark invisible characters in the editor.
    ///
    /// Example: spaces, tabs, carriage returns, etc.
    public editor_invisible: Hsla,
    public editor_wrap_guide: Hsla,
    public editor_active_wrap_guide: Hsla,
    public editor_indent_guide: Hsla,
    public editor_indent_guide_active: Hsla,
    /// Read-access of a symbol, like reading a variable.
    ///
    /// A document highlight is a range inside a text document which deserves
    /// special attention. Usually a document highlight is visualized by changing
    /// the background color of its range.
    public editor_document_highlight_read_background: Hsla,
    /// Read-access of a symbol, like reading a variable.
    ///
    /// A document highlight is a range inside a text document which deserves
    /// special attention. Usually a document highlight is visualized by changing
    /// the background color of its range.
    public editor_document_highlight_write_background: Hsla,

    // ===
    // Terminal
    // ===
    /// Terminal background color.
    public terminal_background: Hsla,
    /// Terminal foreground color.
    public terminal_foreground: Hsla,
    /// Bright terminal foreground color.
    public terminal_bright_foreground: Hsla,
    /// Dim terminal foreground color.
    public terminal_dim_foreground: Hsla,

    /// Black ANSI terminal color.
    public terminal_ansi_black: Hsla,
    /// Bright black ANSI terminal color.
    public terminal_ansi_bright_black: Hsla,
    /// Dim black ANSI terminal color.
    public terminal_ansi_dim_black: Hsla,
    /// Red ANSI terminal color.
    public terminal_ansi_red: Hsla,
    /// Bright red ANSI terminal color.
    public terminal_ansi_bright_red: Hsla,
    /// Dim red ANSI terminal color.
    public terminal_ansi_dim_red: Hsla,
    /// Green ANSI terminal color.
    public terminal_ansi_green: Hsla,
    /// Bright green ANSI terminal color.
    public terminal_ansi_bright_green: Hsla,
    /// Dim green ANSI terminal color.
    public terminal_ansi_dim_green: Hsla,
    /// Yellow ANSI terminal color.
    public terminal_ansi_yellow: Hsla,
    /// Bright yellow ANSI terminal color.
    public terminal_ansi_bright_yellow: Hsla,
    /// Dim yellow ANSI terminal color.
    public terminal_ansi_dim_yellow: Hsla,
    /// Blue ANSI terminal color.
    public terminal_ansi_blue: Hsla,
    /// Bright blue ANSI terminal color.
    public terminal_ansi_bright_blue: Hsla,
    /// Dim blue ANSI terminal color.
    public terminal_ansi_dim_blue: Hsla,
    /// Magenta ANSI terminal color.
    public terminal_ansi_magenta: Hsla,
    /// Bright magenta ANSI terminal color.
    public terminal_ansi_bright_magenta: Hsla,
    /// Dim magenta ANSI terminal color.
    public terminal_ansi_dim_magenta: Hsla,
    /// Cyan ANSI terminal color.
    public terminal_ansi_cyan: Hsla,
    /// Bright cyan ANSI terminal color.
    public terminal_ansi_bright_cyan: Hsla,
    /// Dim cyan ANSI terminal color.
    public terminal_ansi_dim_cyan: Hsla,
    /// White ANSI terminal color.
    public terminal_ansi_white: Hsla,
    /// Bright white ANSI terminal color.
    public terminal_ansi_bright_white: Hsla,
    /// Dim white ANSI terminal color.
    public terminal_ansi_dim_white: Hsla,

    // ===
    // UI/Rich Text
    // ===
    public link_text_hover: Hsla,
}

#[derive(Refineable, Clone)]
public struct ThemeStyles {
    /// The background appearance of the window.
    public window_background_appearance: WindowBackgroundAppearance,
    public system: SystemColors,
    /// An array of colors used for theme elements that iterate through a series of colors.
    ///
    /// Example: Player colors, rainbow brackets and indent guides, etc.
    public accents: AccentColors,

    #[refineable]
    public colors: ThemeColors,

    #[refineable]
    public status: StatusColors,

    public player: PlayerColors,

    public syntax: Arc<SyntaxTheme>,
}

#[cfg(test)]
mod tests {
    use serde_json.json;

    use super.*;

    #[test]
    fn override_a_single_theme_color() {
        let mut colors = ThemeColors.light();

        let magenta: Hsla = gpui.rgb(0xff00ff).into();

        assert_ne!(colors.text, magenta);

        let overrides = ThemeColorsRefinement {
            text: Some(magenta),
            ..Default.default()
        };

        colors.refine(&overrides);

        assert_eq!(colors.text, magenta);
    }

    #[test]
    fn override_multiple_theme_colors() {
        let mut colors = ThemeColors.light();

        let magenta: Hsla = gpui.rgb(0xff00ff).into();
        let green: Hsla = gpui.rgb(0x00ff00).into();

        assert_ne!(colors.text, magenta);
        assert_ne!(colors.background, green);

        let overrides = ThemeColorsRefinement {
            text: Some(magenta),
            background: Some(green),
            ..Default.default()
        };

        colors.refine(&overrides);

        assert_eq!(colors.text, magenta);
        assert_eq!(colors.background, green);
    }

    #[test]
    fn deserialize_theme_colors_refinement_from_json() {
        let colors: ThemeColorsRefinement = serde_json.from_value(json!({
            "background": "#ff00ff",
            "text": "#ff0000"
        }))
        .unwrap();

        assert_eq!(colors.background, Some(gpui.rgb(0xff00ff).into()));
        assert_eq!(colors.text, Some(gpui.rgb(0xff0000).into()));
    }
}
