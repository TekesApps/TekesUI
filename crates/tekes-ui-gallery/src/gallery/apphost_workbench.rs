//! Gallery shell. AppHost is the sole input, layout and window owner.
//! The original renderer remains available through the diagnostic `--legacy` entry.
use super::workbench;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    rc::Rc,
};
use tekes_ui_app_host::{
    AppWindow, ContainerContent, ContainerRenderContext, LocalContainer, NavigationOptions,
    OthersOptions, PresentationOptions,
};
use tekes_ui_controls::{
    CheckState, ScrollArea,
    composition::{
        CheckboxContent, ComponentId, Content, ContentActionPayload, ContentActionRecord,
        ContentHost, InspectionSnapshot, KeyValueTableContent, KeyedSelectableListContent,
        ListContent, ListDisplayUpdate, ListIntentKind, ListRowDisplay, RichContent, RichDocument,
        ScrollContent, SearchFieldContent, StackContent, TextContent, TextFieldContent,
    },
    extended::{
        Axis, KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey, Stack,
    },
};
use tekes_ui_tokens::{Palette, Spacing, proposal::TextRole};

const FILTER: ComponentId = ComponentId(1);
const CHOOSER: ComponentId = ComponentId(3);
const COMMAND: u64 = 0x4757_0000;
const SEARCH_ROUTE: &str = "component-search-field";
const FORM_ROUTE: &str = "component-text-field";
const AREA_ROUTE: &str = "component-text-area";
const COMBO_ROUTE: &str = "component-combo-box";
const SELECT_ROUTE: &str = "component-select";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub page: String,
    pub specimen: Option<String>,
    pub document_id: Option<String>,
}
#[derive(Clone, Copy)]
pub struct Specimen {
    pub key: &'static str,
    pub component_id: ComponentId,
    pub document_id: Option<&'static str>,
    pub label: &'static str,
}
const SEARCH: &[Specimen] = &[Specimen {
    key: "query",
    component_id: ComponentId(8801),
    document_id: Some("tui.search-field"),
    label: "Search query",
}];
const AREA: &[Specimen] = &[Specimen {
    key: "original-area",
    component_id: ComponentId(27502),
    document_id: Some("tui.text-area"),
    label: "Original TextArea Playground",
}];
const COMBO: &[Specimen] = &[Specimen {
    key: "original-combo",
    component_id: ComponentId(27601),
    document_id: Some("tui.combo-box"),
    label: "Original ComboBox Playground",
}];
const SELECT: &[Specimen] = &[Specimen {
    key: "original-select",
    component_id: ComponentId(8901),
    document_id: Some("tui.select"),
    label: "Original Select Playground",
}];
const MARKDOWN: &[Specimen] = &[Specimen {
    key: "original-0",
    component_id: ComponentId(96020),
    document_id: Some("tui.markdown"),
    label: "Original specimen 1",
}];
const CODE_BLOCK: &[Specimen] = &[Specimen {
    key: "original-0",
    component_id: ComponentId(96020),
    document_id: Some("tui.code-block"),
    label: "Original specimen 1",
}];
const DIFF_VIEW: &[Specimen] = &[Specimen {
    key: "original-0",
    component_id: ComponentId(96020),
    document_id: Some("tui.diff-view"),
    label: "Original specimen 1",
}];
const SURFACE: &[Specimen] = &[
    Specimen {
        key: "original-0",
        component_id: ComponentId(96020),
        document_id: Some("tui.surface"),
        label: "Original specimen 1",
    },
    Specimen {
        key: "nested-action",
        component_id: ComponentId(92120),
        document_id: Some("tui.button"),
        label: "Nested real action",
    },
];
const SKELETON: &[Specimen] = &[
    Specimen {
        key: "original-0",
        component_id: ComponentId(96020),
        document_id: Some("tui.skeleton"),
        label: "Original specimen 1",
    },
    Specimen {
        key: "original-1",
        component_id: ComponentId(96021),
        document_id: Some("tui.skeleton"),
        label: "Original specimen 2",
    },
    Specimen {
        key: "original-2",
        component_id: ComponentId(96022),
        document_id: Some("tui.skeleton"),
        label: "Original specimen 3",
    },
    Specimen {
        key: "original-3",
        component_id: ComponentId(96023),
        document_id: Some("tui.skeleton"),
        label: "Original specimen 4",
    },
];
const STATUS_BADGE: &[Specimen] = &[
    Specimen {
        key: "original-0",
        component_id: ComponentId(96020),
        document_id: Some("tui.status-badge"),
        label: "Original specimen 1",
    },
    Specimen {
        key: "original-1",
        component_id: ComponentId(96021),
        document_id: Some("tui.status-badge"),
        label: "Original specimen 2",
    },
    Specimen {
        key: "original-2",
        component_id: ComponentId(96022),
        document_id: Some("tui.status-badge"),
        label: "Original specimen 3",
    },
    Specimen {
        key: "original-3",
        component_id: ComponentId(96023),
        document_id: Some("tui.status-badge"),
        label: "Original specimen 4",
    },
];
const PROGRESS: &[Specimen] = &[
    Specimen {
        key: "original",
        component_id: ComponentId(96020),
        document_id: Some("tui.progress"),
        label: "Live progress",
    },
    Specimen {
        key: "variant-0",
        component_id: ComponentId(96040),
        document_id: Some("tui.progress"),
        label: "0%",
    },
    Specimen {
        key: "variant-1",
        component_id: ComponentId(96041),
        document_id: Some("tui.progress"),
        label: "45%",
    },
    Specimen {
        key: "variant-2",
        component_id: ComponentId(96042),
        document_id: Some("tui.progress"),
        label: "100%",
    },
    Specimen {
        key: "variant-3",
        component_id: ComponentId(96043),
        document_id: Some("tui.progress"),
        label: "Indeterminate",
    },
];
const LINK: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.link"),
    label: "Original link action",
}];
const SEGMENTED: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.segmented-control"),
    label: "Original interval selection",
}];
const BREADCRUMB: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.breadcrumb"),
    label: "Original ancestor navigation",
}];
const TOOLBAR: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.toolbar"),
    label: "Original Create / Rename / Remove actions",
}];
const NUMBER_FIELD: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.number-field"),
    label: "Original bounded stepper",
}];
const ICONS: &[Specimen] = &[
    Specimen {
        key: "catalogue",
        component_id: workbench::apphost_icons::SPECIMEN,
        document_id: Some("tui.icon"),
        label: "Original icon catalogue",
    },
    Specimen {
        key: "query",
        component_id: workbench::apphost_icons::SEARCH,
        document_id: Some("tui.search-field"),
        label: "Original catalogue Search",
    },
];
const TEXT_DOCUMENT: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(99001),
    document_id: Some("tui.text"),
    label: "Original reading surface",
}];
const SCROLL_DOCUMENT: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(99001),
    document_id: Some("tui.scroll-area"),
    label: "Original scroll surface",
}];
const INTERNAL_EDITORS: &[Specimen] = &[
    Specimen {
        key: "single-line",
        component_id: ComponentId(99001),
        document_id: None,
        label: "Original single-line editor",
    },
    Specimen {
        key: "multiline",
        component_id: ComponentId(99002),
        document_id: None,
        label: "Original multiline editor",
    },
];
const FEEDBACK_TOOLTIP: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(103020),
    document_id: Some("tui.tooltip"),
    label: "Original tooltip",
}];
const FEEDBACK_POPOVER: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(103020),
    document_id: Some("tui.popover"),
    label: "Original popover",
}];
const FEEDBACK_TOAST: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(103020),
    document_id: Some("tui.toast"),
    label: "Original toast",
}];
const FEEDBACK_BANNER: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(103020),
    document_id: Some("tui.banner"),
    label: "Original banner",
}];
const FEEDBACK_EMPTY_STATE: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(103020),
    document_id: Some("tui.empty-state"),
    label: "Original empty-state",
}];
const FEEDBACK_MENU: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(103020),
    document_id: Some("tui.menu"),
    label: "Original menu",
}];
const FEEDBACK_DIALOG: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(103020),
    document_id: Some("tui.dialog"),
    label: "Original dialog",
}];
const LAUNCHER: &[Specimen] = &[Specimen {
    key: "open",
    component_id: ComponentId(124020),
    document_id: None,
    label: "Open original AppHost window",
}];
const SPLIT: &[Specimen] = &[
    Specimen {
        key: "original",
        component_id: ComponentId(92020),
        document_id: Some("tui.split-view"),
        label: "Original SplitView",
    },
    Specimen {
        key: "divider",
        component_id: ComponentId(123003),
        document_id: Some("tui.split-view"),
        label: "Original split divider",
    },
];
const ICON_BUTTON: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.icon-button"),
    label: "Original named icon Button",
}];
const BRAND: &[Specimen] = &[
    Specimen {
        key: "current",
        component_id: ComponentId(120010),
        document_id: None,
        label: "Current applied brand",
    },
    Specimen {
        key: "candidate",
        component_id: ComponentId(120020),
        document_id: None,
        label: "Candidate brand",
    },
];
const COMMAND_PALETTE: &[Specimen] = &[
    Specimen {
        key: "original",
        component_id: ComponentId(92020),
        document_id: Some("tui.command-palette"),
        label: "Original CommandPalette",
    },
    Specimen {
        key: "query",
        component_id: ComponentId(121001),
        document_id: Some("tui.text-field"),
        label: "Original command query",
    },
];
const PATTERN_INPUT: &[Specimen] = &[Specimen {
    key: "input",
    component_id: ComponentId(27501),
    document_id: Some("tui.text-field"),
    label: "Original pattern input",
}];
const MASTER: &[Specimen] = &[Specimen {
    key: "list",
    component_id: ComponentId(27403),
    document_id: Some("tui.list"),
    label: "Original document list",
}];
const DANGER: &[Specimen] = &[Specimen {
    key: "dialog",
    component_id: ComponentId(110006),
    document_id: Some("tui.dialog"),
    label: "Original destructive confirmation",
}];
const FIELD_META: &[Specimen] = &[Specimen {
    key: "field",
    component_id: ComponentId(27501),
    document_id: Some("tui.field"),
    label: "Original Field metadata and editor",
}];
const LIST: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.list"),
    label: "Original List",
}];
const TREE: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.tree"),
    label: "Original Tree",
}];
const TABLE: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.data-table"),
    label: "Original DataTable",
}];
const CHART: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.chart"),
    label: "Original Chart",
}];
const TABS: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.tabs"),
    label: "Original Tabs",
}];
const DISCLOSURE: &[Specimen] = &[Specimen {
    key: "original",
    component_id: ComponentId(92020),
    document_id: Some("tui.disclosure"),
    label: "Original Disclosure",
}];
const CALIBRATION: &[Specimen] = &[Specimen {
    key: "original",
    component_id: workbench::apphost_text_pages::CALIBRATION,
    document_id: None,
    label: "Original StaticText calibration",
}];
const GEOMETRY: &[Specimen] = &[Specimen {
    key: "original-reference",
    component_id: workbench::apphost_quality::PREVIEW,
    document_id: None,
    label: "Original raster alignment reference",
}];
const MOTION: &[Specimen] = &[Specimen {
    key: "original-motion",
    component_id: ComponentId(97020),
    document_id: None,
    label: "Original Motion preview",
}];
const FOUNDATIONS: &[Specimen] = &[Specimen {
    key: "original-reference",
    component_id: ComponentId(97804),
    document_id: None,
    label: "Original foundation reference",
}];
const SCENE3D: &[Specimen] = &[Specimen {
    key: "original-scene3d",
    component_id: workbench::apphost_scene3d::SPECIMEN,
    document_id: None,
    label: "Original Scene3D preview",
}];
const SWITCH: &[Specimen] = &[
    Specimen {
        key: "original",
        component_id: ComponentId(92020),
        document_id: Some("tui.switch"),
        label: "Original Playground",
    },
    Specimen {
        key: "matrix-0",
        component_id: ComponentId(92801),
        document_id: Some("tui.switch"),
        label: "State fixture 1",
    },
    Specimen {
        key: "matrix-1",
        component_id: ComponentId(92803),
        document_id: Some("tui.switch"),
        label: "State fixture 2",
    },
    Specimen {
        key: "matrix-2",
        component_id: ComponentId(92805),
        document_id: Some("tui.switch"),
        label: "State fixture 3",
    },
    Specimen {
        key: "matrix-3",
        component_id: ComponentId(92807),
        document_id: Some("tui.switch"),
        label: "State fixture 4",
    },
    Specimen {
        key: "matrix-4",
        component_id: ComponentId(92809),
        document_id: Some("tui.switch"),
        label: "State fixture 5",
    },
    Specimen {
        key: "matrix-5",
        component_id: ComponentId(92811),
        document_id: Some("tui.switch"),
        label: "State fixture 6",
    },
    Specimen {
        key: "matrix-6",
        component_id: ComponentId(92813),
        document_id: Some("tui.switch"),
        label: "State fixture 7",
    },
    Specimen {
        key: "matrix-7",
        component_id: ComponentId(92815),
        document_id: Some("tui.switch"),
        label: "State fixture 8",
    },
    Specimen {
        key: "matrix-8",
        component_id: ComponentId(92817),
        document_id: Some("tui.switch"),
        label: "State fixture 9",
    },
    Specimen {
        key: "matrix-9",
        component_id: ComponentId(92819),
        document_id: Some("tui.switch"),
        label: "State fixture 10",
    },
];
const SLIDER: &[Specimen] = &[
    Specimen {
        key: "original",
        component_id: ComponentId(92020),
        document_id: Some("tui.slider"),
        label: "Original Playground",
    },
    Specimen {
        key: "matrix-0",
        component_id: ComponentId(92801),
        document_id: Some("tui.slider"),
        label: "State fixture 1",
    },
    Specimen {
        key: "matrix-1",
        component_id: ComponentId(92803),
        document_id: Some("tui.slider"),
        label: "State fixture 2",
    },
    Specimen {
        key: "matrix-2",
        component_id: ComponentId(92805),
        document_id: Some("tui.slider"),
        label: "State fixture 3",
    },
    Specimen {
        key: "matrix-3",
        component_id: ComponentId(92807),
        document_id: Some("tui.slider"),
        label: "State fixture 4",
    },
    Specimen {
        key: "matrix-4",
        component_id: ComponentId(92809),
        document_id: Some("tui.slider"),
        label: "State fixture 5",
    },
    Specimen {
        key: "matrix-5",
        component_id: ComponentId(92811),
        document_id: Some("tui.slider"),
        label: "State fixture 6",
    },
];
const RADIO: &[Specimen] = &[
    Specimen {
        key: "original",
        component_id: ComponentId(92020),
        document_id: Some("tui.radio-group"),
        label: "Original Playground",
    },
    Specimen {
        key: "matrix-0",
        component_id: ComponentId(92801),
        document_id: Some("tui.radio-group"),
        label: "State fixture 1",
    },
    Specimen {
        key: "matrix-1",
        component_id: ComponentId(92803),
        document_id: Some("tui.radio-group"),
        label: "State fixture 2",
    },
    Specimen {
        key: "matrix-2",
        component_id: ComponentId(92805),
        document_id: Some("tui.radio-group"),
        label: "State fixture 3",
    },
    Specimen {
        key: "matrix-3",
        component_id: ComponentId(92807),
        document_id: Some("tui.radio-group"),
        label: "State fixture 4",
    },
    Specimen {
        key: "matrix-4",
        component_id: ComponentId(92809),
        document_id: Some("tui.radio-group"),
        label: "State fixture 5",
    },
    Specimen {
        key: "matrix-5",
        component_id: ComponentId(92811),
        document_id: Some("tui.radio-group"),
        label: "State fixture 6",
    },
    Specimen {
        key: "matrix-6",
        component_id: ComponentId(92813),
        document_id: Some("tui.radio-group"),
        label: "State fixture 7",
    },
    Specimen {
        key: "matrix-7",
        component_id: ComponentId(92815),
        document_id: Some("tui.radio-group"),
        label: "State fixture 8",
    },
    Specimen {
        key: "matrix-8",
        component_id: ComponentId(92817),
        document_id: Some("tui.radio-group"),
        label: "State fixture 9",
    },
    Specimen {
        key: "matrix-9",
        component_id: ComponentId(92819),
        document_id: Some("tui.radio-group"),
        label: "State fixture 10",
    },
];
const STACK: &[Specimen] = &[
    Specimen {
        key: "gap0",
        component_id: ComponentId(92600),
        document_id: Some("tui.stack"),
        label: "Gap 0",
    },
    Specimen {
        key: "gap8",
        component_id: ComponentId(92601),
        document_id: Some("tui.stack"),
        label: "Gap 8",
    },
    Specimen {
        key: "gap16",
        component_id: ComponentId(92602),
        document_id: Some("tui.stack"),
        label: "Gap 16",
    },
];
const GRID: &[Specimen] = &[
    Specimen {
        key: "columns2",
        component_id: ComponentId(92600),
        document_id: Some("tui.grid"),
        label: "2 columns",
    },
    Specimen {
        key: "columns3",
        component_id: ComponentId(92601),
        document_id: Some("tui.grid"),
        label: "3 columns",
    },
    Specimen {
        key: "columns4",
        component_id: ComponentId(92602),
        document_id: Some("tui.grid"),
        label: "4 columns",
    },
];
const SEPARATOR: &[Specimen] = &[
    Specimen {
        key: "horizontal",
        component_id: ComponentId(92600),
        document_id: Some("tui.separator"),
        label: "Horizontal",
    },
    Specimen {
        key: "vertical",
        component_id: ComponentId(92601),
        document_id: Some("tui.separator"),
        label: "Vertical",
    },
];
const BUTTON: &[Specimen] = &[
    Specimen {
        key: "original-button",
        component_id: ComponentId(92020),
        document_id: Some("tui.button"),
        label: "Original Button Playground",
    },
    Specimen {
        key: "matrix-default",
        component_id: ComponentId(92301),
        document_id: Some("tui.button"),
        label: "State: default",
    },
    Specimen {
        key: "matrix-hover",
        component_id: ComponentId(92303),
        document_id: Some("tui.button"),
        label: "State: hover",
    },
    Specimen {
        key: "matrix-pressed",
        component_id: ComponentId(92305),
        document_id: Some("tui.button"),
        label: "State: pressed",
    },
    Specimen {
        key: "matrix-focus-visible",
        component_id: ComponentId(92307),
        document_id: Some("tui.button"),
        label: "State: focus-visible",
    },
    Specimen {
        key: "matrix-disabled",
        component_id: ComponentId(92309),
        document_id: Some("tui.button"),
        label: "State: disabled",
    },
    Specimen {
        key: "matrix-loading",
        component_id: ComponentId(92311),
        document_id: Some("tui.button"),
        label: "State: loading",
    },
    Specimen {
        key: "variant-small-primary",
        component_id: ComponentId(92400),
        document_id: Some("tui.button"),
        label: "small-primary",
    },
    Specimen {
        key: "variant-small-quiet",
        component_id: ComponentId(92401),
        document_id: Some("tui.button"),
        label: "small-quiet",
    },
    Specimen {
        key: "variant-medium-primary",
        component_id: ComponentId(92402),
        document_id: Some("tui.button"),
        label: "medium-primary",
    },
    Specimen {
        key: "variant-medium-quiet",
        component_id: ComponentId(92403),
        document_id: Some("tui.button"),
        label: "medium-quiet",
    },
    Specimen {
        key: "variant-large-primary",
        component_id: ComponentId(92404),
        document_id: Some("tui.button"),
        label: "large-primary",
    },
    Specimen {
        key: "variant-large-quiet",
        component_id: ComponentId(92405),
        document_id: Some("tui.button"),
        label: "large-quiet",
    },
];
const CHECKBOX: &[Specimen] = &[
    Specimen {
        key: "original-checkbox",
        component_id: ComponentId(92020),
        document_id: Some("tui.checkbox"),
        label: "Original Checkbox Playground",
    },
    Specimen {
        key: "matrix-unchecked",
        component_id: ComponentId(92301),
        document_id: Some("tui.checkbox"),
        label: "Unchecked",
    },
    Specimen {
        key: "matrix-unchecked-hover",
        component_id: ComponentId(92303),
        document_id: Some("tui.checkbox"),
        label: "Unchecked · hover",
    },
    Specimen {
        key: "matrix-unchecked-pressed",
        component_id: ComponentId(92305),
        document_id: Some("tui.checkbox"),
        label: "Unchecked · pressed",
    },
    Specimen {
        key: "matrix-unchecked-focus",
        component_id: ComponentId(92307),
        document_id: Some("tui.checkbox"),
        label: "Unchecked · focus-visible",
    },
    Specimen {
        key: "matrix-unchecked-disabled",
        component_id: ComponentId(92309),
        document_id: Some("tui.checkbox"),
        label: "Unchecked · disabled",
    },
    Specimen {
        key: "matrix-unchecked-error-focus",
        component_id: ComponentId(92311),
        document_id: Some("tui.checkbox"),
        label: "Unchecked · error + focus-visible",
    },
    Specimen {
        key: "matrix-checked-error",
        component_id: ComponentId(92313),
        document_id: Some("tui.checkbox"),
        label: "Checked · error",
    },
    Specimen {
        key: "matrix-mixed-error-disabled",
        component_id: ComponentId(92315),
        document_id: Some("tui.checkbox"),
        label: "Indeterminate · error + disabled",
    },
    Specimen {
        key: "matrix-checked",
        component_id: ComponentId(92317),
        document_id: Some("tui.checkbox"),
        label: "Checked",
    },
    Specimen {
        key: "matrix-checked-pressed",
        component_id: ComponentId(92319),
        document_id: Some("tui.checkbox"),
        label: "Checked · pressed",
    },
    Specimen {
        key: "matrix-checked-hover",
        component_id: ComponentId(92321),
        document_id: Some("tui.checkbox"),
        label: "Checked · hover",
    },
    Specimen {
        key: "matrix-checked-focus",
        component_id: ComponentId(92323),
        document_id: Some("tui.checkbox"),
        label: "Checked · focus-visible",
    },
    Specimen {
        key: "matrix-mixed",
        component_id: ComponentId(92325),
        document_id: Some("tui.checkbox"),
        label: "Indeterminate",
    },
    Specimen {
        key: "matrix-mixed-focus",
        component_id: ComponentId(92327),
        document_id: Some("tui.checkbox"),
        label: "Indeterminate · focus-visible",
    },
    Specimen {
        key: "matrix-checked-disabled",
        component_id: ComponentId(92329),
        document_id: Some("tui.checkbox"),
        label: "Checked · disabled",
    },
    Specimen {
        key: "matrix-mixed-disabled",
        component_id: ComponentId(92331),
        document_id: Some("tui.checkbox"),
        label: "Indeterminate · disabled",
    },
];
const FORM: &[Specimen] = &[
    Specimen {
        key: "original-field",
        component_id: ComponentId(27501),
        document_id: Some("tui.text-field"),
        label: "Original TextField Playground",
    },
    Specimen {
        key: "name",
        component_id: ComponentId(100),
        document_id: Some("tui.text-field"),
        label: "Project name",
    },
    Specimen {
        key: "owner",
        component_id: ComponentId(102),
        document_id: Some("tui.text-field"),
        label: "Project owner",
    },
    Specimen {
        key: "published",
        component_id: ComponentId(101),
        document_id: Some("tui.checkbox"),
        label: "Published",
    },
];
fn quality_specimens(page: &str) -> Option<&'static [Specimen]> {
    static ITEMS: std::sync::OnceLock<HashMap<&'static str, Vec<Specimen>>> =
        std::sync::OnceLock::new();
    ITEMS
        .get_or_init(|| {
            workbench::apphost_quality::GROUPS
                .into_iter()
                .map(|(slug, category, _)| {
                    let group = super::catalog::group(category);
                    let specimens = group
                        .component_ids
                        .iter()
                        .map(|id| Specimen {
                            key: id.as_str(),
                            component_id: workbench::apphost_quality::PREVIEW,
                            document_id: Some(id.as_str()),
                            label: super::catalog::component(id).name.as_str(),
                        })
                        .collect();
                    (slug, specimens)
                })
                .collect()
        })
        .get(page)
        .map(Vec::as_slice)
}
/// Virtual showcase children remain under the original consistency directory row.
fn directory_page(page: &str) -> &str {
    if workbench::apphost_quality::category_for_slug(page).is_some() {
        "lab-consistency"
    } else {
        page
    }
}
fn known_page(page: &str) -> bool {
    workbench::entries()
        .iter()
        .any(|entry| entry.slug() == page)
        || workbench::apphost_quality::category_for_slug(page).is_some()
}
pub fn specimens(page: &str) -> &'static [Specimen] {
    if let Some(items) = quality_specimens(page) {
        return items;
    }
    match page {
        SEARCH_ROUTE => SEARCH,
        FORM_ROUTE => FORM,
        AREA_ROUTE => AREA,
        COMBO_ROUTE => COMBO,
        SELECT_ROUTE => SELECT,
        "component-checkbox" => CHECKBOX,
        "component-button" => BUTTON,
        "component-stack" => STACK,
        "component-grid" => GRID,
        "component-separator" => SEPARATOR,
        "component-switch" => SWITCH,
        "component-icon" => ICONS,
        "component-text" => TEXT_DOCUMENT,
        "component-scroll-area" => SCROLL_DOCUMENT,
        "lab-internal-text" => INTERNAL_EDITORS,
        "lab-calibration" => CALIBRATION,
        "lab-geometry" => GEOMETRY,
        "component-breadcrumb" => BREADCRUMB,
        "component-tooltip" => FEEDBACK_TOOLTIP,
        "component-popover" => FEEDBACK_POPOVER,
        "component-toast" => FEEDBACK_TOAST,
        "component-banner" => FEEDBACK_BANNER,
        "component-empty-state" => FEEDBACK_EMPTY_STATE,
        "component-menu" => FEEDBACK_MENU,
        "component-dialog" => FEEDBACK_DIALOG,
        "apphost" => LAUNCHER,
        "component-split-view" => SPLIT,
        "component-icon-button" => ICON_BUTTON,
        "lab-matrix" => BRAND,
        "component-command-palette" => COMMAND_PALETTE,
        "component-field" => FIELD_META,
        "pattern-form" | "pattern-filter" | "pattern-async" | "example-settings" => PATTERN_INPUT,
        "pattern-master" => MASTER,
        "pattern-danger" => DANGER,
        "component-list" => LIST,
        "component-tree" => TREE,
        "component-data-table" => TABLE,
        "component-chart" => CHART,
        "component-tabs" => TABS,
        "component-disclosure" => DISCLOSURE,
        "component-toolbar" => TOOLBAR,
        "component-number-field" => NUMBER_FIELD,
        "component-link" => LINK,
        "component-segmented-control" => SEGMENTED,
        "component-scene3d" => SCENE3D,
        "foundation-motion" => MOTION,
        "foundation-tokens"
        | "foundation-spacing"
        | "foundation-shape"
        | "foundation-accessibility"
        | "foundation-typography"
        | "foundation-iconography"
        | "lab-a11y"
        | "lab-stress" => FOUNDATIONS,
        "component-markdown" => MARKDOWN,
        "component-code-block" => CODE_BLOCK,
        "component-diff-view" => DIFF_VIEW,
        "component-surface" => SURFACE,
        "component-skeleton" => SKELETON,
        "component-status-badge" => STATUS_BADGE,
        "component-progress" => PROGRESS,

        "component-slider" => SLIDER,
        "component-radio-group" => RADIO,
        _ => &[],
    }
}
impl Selection {
    fn new(page: &str, specimen: Option<&str>) -> Self {
        let available = specimens(page);
        let selected = specimen
            .and_then(|key| available.iter().find(|item| item.key == key))
            .or_else(|| (specimen.is_none() && available.len() == 1).then(|| &available[0]));
        Self {
            page: page.into(),
            specimen: selected.map(|item| item.key.into()),
            document_id: selected.and_then(|item| item.document_id.map(str::to_owned)),
        }
    }
}
/// Shared by the Focus and the two read-only panels. A host is never cloned.
/// Panels read the selection and an observational snapshot of its live host.
pub struct WorkbenchState {
    settings: Option<tekes_ui_app_host::SettingsOwner>,
    window_requests: usize,
    palette: Rc<Cell<Palette>>,
    current: u64,
    visits: Vec<Selection>,
    pages: HashMap<String, Rc<RefCell<MountedPage>>>,
    pending: VecDeque<tekes_ui_app_host::navigation::Entry>,
    inspected: Option<(String, ComponentId, u64)>,
    document_return_requested: bool,
    session_dirs: Vec<super::apphost::SessionTempDir>,
}
pub type SharedWorkbench = Rc<RefCell<WorkbenchState>>;
impl WorkbenchState {
    pub fn selection(&self) -> &Selection {
        &self.visits[self.current as usize]
    }
    #[cfg(test)]
    fn with_selected_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost, Specimen)) {
        let selection = self.selection().clone();
        if let Some(specimen) = specimens(&selection.page)
            .iter()
            .find(|item| Some(item.key) == selection.specimen.as_deref())
        {
            visit(
                &mut self
                    .pages
                    .get(&selection.page)
                    .expect("current page mounted")
                    .borrow_mut(),
                *specimen,
            );
        }
    }
    // Commands run inside a borrowed Focus host. Apply recording changes at the
    // next host operation, after that borrow ends, without replaying any event.
    fn sync_inspection(&mut self) {
        let selection = self.selection();
        let target = specimens(&selection.page)
            .iter()
            .find(|item| Some(item.key) == selection.specimen.as_deref())
            .and_then(|item| {
                self.pages[&selection.page]
                    .borrow()
                    .get(item.component_id)
                    .map(|node| {
                        (
                            selection.page.clone(),
                            item.component_id,
                            node.mount_identity(),
                        )
                    })
            });
        if target == self.inspected {
            return;
        }
        if let Some((page, id, _)) = self.inspected.take() {
            self.pages[&page]
                .borrow_mut()
                .set_inspection_enabled(id, false);
        }
        if let Some((page, id, _)) = &target {
            self.pages[page]
                .borrow_mut()
                .set_inspection_enabled(*id, true);
        }
        self.inspected = target;
    }
    fn selected_snapshot(&self) -> Option<InspectionSnapshot> {
        let selection = self.selection();
        let specimen = specimens(&selection.page)
            .iter()
            .find(|item| Some(item.key) == selection.specimen.as_deref())?;
        self.pages[&selection.page]
            .borrow()
            .inspection_snapshot(specimen.component_id)
    }
    fn entry(&self, visit: u64) -> tekes_ui_app_host::navigation::Entry {
        let selection = &self.visits[visit as usize];
        tekes_ui_app_host::navigation::Entry {
            id: format!("gallery-visit-{visit}"),
            route: selection.page.clone(),
            params: serde_json::json!({"visit": visit, "specimen": selection.specimen, "document_id": selection.document_id}),
        }
    }
    fn navigate(&mut self, page: &str) {
        if self.selection().page == page {
            return;
        }
        // Each visit retains its own chooser selection, including repeated routes.
        let previous = self
            .visits
            .iter()
            .rev()
            .find(|selection| selection.page == page);
        let selection = Selection::new(
            page,
            previous.and_then(|selection| selection.specimen.as_deref()),
        );
        let visit = self.visits.len() as u64;
        self.visits.push(selection);
        self.pending.push_back(self.entry(visit));
    }
    fn select(&mut self, key: Option<&str>) {
        let page = self.selection().page.clone();
        self.visits[self.current as usize] = if key.is_none() {
            Selection {
                page,
                specimen: None,
                document_id: None,
            }
        } else {
            Selection::new(&page, key)
        };
    }
    fn restore(&mut self, command: u64) -> anyhow::Result<()> {
        let visit = command
            .checked_sub(COMMAND)
            .filter(|visit| (*visit as usize) < self.visits.len())
            .ok_or_else(|| anyhow::anyhow!("unknown Gallery visit"))?;
        let page = self.visits[visit as usize].page.clone();
        if !self.pages.contains_key(&page) {
            self.pages.insert(
                page.clone(),
                Rc::new(RefCell::new(page_host(&page, &self.palette)?)),
            );
        }
        self.current = visit;
        Ok(())
    }
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn stack(id: u64, children: Vec<Content>) -> Content {
    Content::new(
        ComponentId(id),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            children,
            flexible_child: None,
        },
    )
}
/// Original hosted stories keep their owning StoryState; plain pages keep the
/// previously accepted Host. Both expose exactly one Host to AppHost.
enum MountedPage {
    Brand(Box<workbench::apphost_brand::BrandStory>),
    Feedback(Box<workbench::apphost_feedback::FeedbackStory>),
    Patterns(Box<workbench::apphost_patterns::AppHostPatterns>),
    Plain(Box<ContentHost>),
    Story(Box<workbench::apphost_story::AppHostStory>),
    Basic(Box<workbench::apphost_basic::BasicStory>),
    Scene3D(Box<workbench::apphost_scene3d::AppHostScene3D>),
    Motion(Box<workbench::apphost_motion::AppHostMotion>),
    Icons(Box<workbench::apphost_icons::AppHostIcons>),
    Documents(Box<workbench::apphost_documents::AppHostDocuments>),
    Foundations(Box<workbench::apphost_foundations::AppHostFoundations>),
}
impl std::ops::Deref for MountedPage {
    type Target = ContentHost;
    fn deref(&self) -> &ContentHost {
        match self {
            Self::Brand(story) => &story.host,
            Self::Feedback(story) => &story.host,
            Self::Patterns(story) => story.host(),
            Self::Plain(host) => host,
            Self::Story(story) => story.host(),
            Self::Basic(story) => &story.host,
            Self::Scene3D(story) => &story.host,
            Self::Motion(story) => &story.host,
            Self::Icons(story) => &story.host,
            Self::Documents(story) => &story.host,
            Self::Foundations(story) => &story.host,
        }
    }
}
impl std::ops::DerefMut for MountedPage {
    fn deref_mut(&mut self) -> &mut ContentHost {
        match self {
            Self::Brand(story) => &mut story.host,
            Self::Feedback(story) => &mut story.host,
            Self::Patterns(story) => story.host_mut(),
            Self::Plain(host) => host,
            Self::Story(story) => story.host_mut(),
            Self::Basic(story) => &mut story.host,
            Self::Scene3D(story) => &mut story.host,
            Self::Motion(story) => &mut story.host,
            Self::Icons(story) => &mut story.host,
            Self::Documents(story) => &mut story.host,
            Self::Foundations(story) => &mut story.host,
        }
    }
}
impl From<ContentHost> for MountedPage {
    fn from(host: ContentHost) -> Self {
        Self::Plain(Box::new(host))
    }
}
/// #291 composition fixture remains separately labelled inside the Field page.
pub(super) fn composition_fixture() -> Content {
    Content::new(
        ComponentId(200),
        ListContent::scrollable(
            "Composition example",
            ComponentId(201),
            vec![
                text(
                    202,
                    "Additional composition fixture — not the original Playground",
                ),
                Content::new(
                    ComponentId(100),
                    TextFieldContent::new("Project name", "Gallery"),
                ),
                Content::new(
                    ComponentId(102),
                    TextFieldContent::new("Project owner", "TekesUI team"),
                ),
                Content::new(
                    ComponentId(101),
                    CheckboxContent::new("Published", CheckState::Unchecked),
                ),
            ],
        ),
    )
}
fn page_host(page: &str, palette: &Rc<Cell<Palette>>) -> anyhow::Result<MountedPage> {
    if page == "lab-matrix" {
        return Ok(MountedPage::Brand(Box::new(
            workbench::apphost_brand::BrandStory::new(palette.clone()),
        )));
    }
    if let Some(category) = workbench::apphost_quality::category_for_slug(page) {
        return Ok(workbench::apphost_quality::host_for(category)?.into());
    }

    let entry = workbench::entries()
        .iter()
        .find(|entry| entry.slug() == page)
        .ok_or_else(|| anyhow::anyhow!("unknown Gallery route: {page}"))?;
    if matches!(
        entry.renderer(),
        "tokens"
            | "spacing"
            | "shape"
            | "accessibility"
            | "typography"
            | "iconography"
            | "a11y"
            | "stress"
    ) {
        return Ok(MountedPage::Foundations(Box::new(
            workbench::apphost_foundations::AppHostFoundations::new(entry)?,
        )));
    }
    if entry.renderer() == "icon" {
        return Ok(MountedPage::Icons(Box::new(
            workbench::apphost_icons::AppHostIcons::new(entry)?,
        )));
    }
    if workbench::apphost_text_pages::supports(entry.renderer())
        || entry.renderer() == "calibration"
    {
        return Ok(MountedPage::Documents(Box::new(
            workbench::apphost_documents::AppHostDocuments::new(entry)?,
        )));
    }
    if matches!(
        entry.renderer(),
        "form" | "filter" | "master" | "async" | "danger" | "settings" | "field-meta"
    ) {
        return Ok(MountedPage::Patterns(Box::new(
            workbench::apphost_patterns::AppHostPatterns::new(entry)?,
        )));
    }
    if workbench::apphost_feedback::supports(entry.renderer()) {
        return Ok(MountedPage::Feedback(Box::new(
            workbench::apphost_feedback::FeedbackStory::new(entry),
        )));
    }
    if entry.renderer() == "apphost" {
        let mut host = ContentHost::new(Content::new(
            ComponentId(124000),
            ListContent::scrollable(
                entry.title(),
                ComponentId(124001),
                vec![
                    workbench::story_heading(124002, entry.title()),
                    text(124003, workbench::story_docs(entry)),
                    text(
                        124004,
                        format!(
                            "{}\n{}",
                            workbench::api_reference(entry),
                            "Open the original AppHost demonstration in its own window."
                        ),
                    ),
                    Content::new(
                        ComponentId(124020),
                        tekes_ui_controls::composition::Button::new("Open AppHost demonstration"),
                    ),
                ],
            ),
        ))
        .map_err(|error| anyhow::anyhow!("AppHost launcher: {error:?}"))?;
        host.set_action_events(true);
        return Ok(host.into());
    }
    if entry.renderer() == "consistency" {
        return Ok(workbench::apphost_quality::index_host(entry)?.into());
    }
    if entry.renderer() == "geometry" {
        return Ok(workbench::apphost_quality::geometry_host(entry)?.into());
    }
    if page == "foundation-motion" {
        return Ok(MountedPage::Motion(Box::new(
            workbench::apphost_motion::AppHostMotion::new(entry)?,
        )));
    }
    if page == "component-scene3d" {
        return Ok(MountedPage::Scene3D(Box::new(
            workbench::apphost_scene3d::AppHostScene3D::new(entry)?,
        )));
    }
    if matches!(entry.renderer(), "docs" | "index") {
        let mut children = vec![
            workbench::story_heading(93001, entry.title()),
            text(93002, workbench::story_docs(entry)),
        ];
        if entry.renderer() == "docs" {
            children.push(text(93003, workbench::overview_copy(entry)));
        } else {
            children.extend(
                workbench::entries()
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| !item.component().is_empty())
                    .map(|(i, item)| {
                        Content::new(
                            ComponentId(94000 + i as u64),
                            tekes_ui_controls::composition::Button::new(format!(
                                "{}{}",
                                item.title(),
                                if item.planned() { " · planned" } else { "" }
                            )),
                        )
                    }),
            );
        }
        let mut host = ContentHost::new(Content::new(
            ComponentId(93000),
            ListContent::scrollable(entry.title(), ComponentId(93004), children)
                .content_padding(16.),
        ))
        .map_err(|error| anyhow::anyhow!("Gallery reference: {error:?}"))?;
        host.set_action_events(true);
        return Ok(host.into());
    }
    if matches!(
        page,
        "component-checkbox"
            | "component-button"
            | "component-stack"
            | "component-grid"
            | "component-separator"
            | "component-link"
            | "component-segmented-control"
            | "component-breadcrumb"
            | "component-list"
            | "component-tree"
            | "component-data-table"
            | "component-chart"
            | "component-split-view"
            | "component-icon-button"
            | "component-command-palette"
            | "component-tabs"
            | "component-disclosure"
            | "component-toolbar"
            | "component-number-field"
            | "component-switch"
            | "component-slider"
            | "component-radio-group"
            | "component-markdown"
            | "component-code-block"
            | "component-diff-view"
            | "component-surface"
            | "component-skeleton"
            | "component-status-badge"
            | "component-progress"
    ) {
        return Ok(MountedPage::Basic(Box::new(
            workbench::apphost_basic::BasicStory::new(entry),
        )));
    }
    if workbench::apphost_story::supports(page) {
        return Ok(MountedPage::Story(Box::new(
            workbench::apphost_story::AppHostStory::new(entry),
        )));
    }
    let content = stack(
        200,
        vec![
            text(201, entry.title()),
            text(
                202,
                "This route has not migrated to AppHost yet. Open the default Gallery to use its existing stories.",
            ),
        ],
    );
    let mut host =
        ContentHost::new(content).map_err(|error| anyhow::anyhow!("Gallery page: {error:?}"))?;
    host.set_action_events(true);
    Ok(host.into())
}
struct FocusContent(SharedWorkbench);
impl FocusContent {
    fn current_page(&self) -> Rc<RefCell<MountedPage>> {
        let state = self.0.borrow();
        state
            .pages
            .get(&state.selection().page)
            .expect("current page mounted")
            .clone()
    }
}
impl ContainerContent for FocusContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        self.0.borrow_mut().sync_inspection();
        let visited_page = self.0.borrow().selection().page.clone();
        let index_page = visited_page == "component-index";
        let consistency_page = visited_page == "lab-consistency";
        let launcher_page = visited_page == "apphost";
        let host = self.current_page();
        let mut page = host.borrow_mut();
        visit(&mut page);
        if self.0.borrow().selection().page != visited_page {
            // Navigation commands execute while borrowing this departing Host.
            // Stop it here; borrowing it again through WorkbenchState would panic.
            page.set_window_active(false);
        }
        match &mut *page {
            MountedPage::Brand(story) => story.after_host_visit(),
            MountedPage::Feedback(story) => story.after_host_visit(),
            MountedPage::Patterns(story) => story.after_host_visit(),
            MountedPage::Story(story) => story.after_host_visit(),
            MountedPage::Basic(story) => story.after_host_visit(),
            MountedPage::Scene3D(story) => story.after_host_visit(),
            MountedPage::Motion(story) => story.after_host_visit(),
            MountedPage::Icons(story) => story.after_host_visit(),
            MountedPage::Documents(story) => story.after_host_visit(),
            MountedPage::Foundations(story) => story.after_host_visit(),
            MountedPage::Plain(_) => {}
        }
        if index_page || consistency_page || launcher_page {
            for record in page.drain_action_records() {
                if record.action.kind != tekes_ui_controls::composition::ContentActionKind::Activate
                {
                    continue;
                }
                if launcher_page {
                    if record.action.target == ComponentId(124020) {
                        self.0.borrow_mut().window_requests += 1;
                    }
                } else if consistency_page {
                    if let Some(destination) =
                        workbench::apphost_quality::destination(record.action.target)
                    {
                        self.0.borrow_mut().navigate(destination);
                    }
                } else if let Some(entry) = record
                    .action
                    .target
                    .0
                    .checked_sub(94000)
                    .and_then(|i| workbench::entries().get(i as usize))
                {
                    self.0.borrow_mut().navigate(entry.slug());
                }
            }
        }
    }
    fn take_window_open_request(
        &mut self,
        theme: tekes_ui_tokens::Theme,
    ) -> anyhow::Result<Option<tekes_ui_app_host::WindowOpenRequest>> {
        let settings = {
            let mut state = self.0.borrow_mut();
            if state.window_requests == 0 {
                return Ok(None);
            }
            state.window_requests -= 1;
            state
                .settings
                .as_ref()
                .expect("assembled Gallery Settings owner")
                .clone()
        };
        let (request, session_dir) = super::apphost::open_request(theme, settings)?;
        self.0.borrow_mut().session_dirs.push(session_dir);
        Ok(Some(request))
    }
    fn prepare_resources(
        &mut self,
        context: &mut ContainerRenderContext<'_>,
    ) -> anyhow::Result<()> {
        match &mut *self.current_page().borrow_mut() {
            MountedPage::Scene3D(story) => story.prepare_resources(context)?,
            MountedPage::Motion(story) => story.prepare_resources(context)?,
            MountedPage::Icons(story) => story.prepare_resources(context)?,
            MountedPage::Documents(story) => story.prepare_resources(context)?,
            _ => {}
        }
        Ok(())
    }
    fn finish_resources(&mut self, context: &mut ContainerRenderContext<'_>) -> anyhow::Result<()> {
        match &mut *self.current_page().borrow_mut() {
            MountedPage::Scene3D(story) => story.finish_resources(context)?,
            MountedPage::Motion(story) => story.finish_resources(context)?,
            MountedPage::Icons(story) => story.finish_resources(context)?,
            _ => {}
        }
        Ok(())
    }
    fn take_navigation_request(&mut self) -> Option<tekes_ui_app_host::navigation::Entry> {
        self.0.borrow_mut().pending.pop_front()
    }
}
fn navigation_sections(query: &str) -> Vec<super::navigation::Section> {
    if query.trim().is_empty() {
        super::navigation::sections().to_vec()
    } else {
        super::navigation::Navigation::filter_sections(query)
    }
}
fn rows(snapshot: &KeyedListSnapshot, base: u64) -> Vec<ListRowDisplay> {
    snapshot
        .items
        .iter()
        .map(|item| ListRowDisplay {
            key: item.key,
            row_id: ComponentId(base + item.key.0),
            display: ListDisplayUpdate::Keep,
        })
        .collect()
}
fn list(name: &str, snapshot: &KeyedListSnapshot, base: u64) -> KeyedSelectableListContent {
    KeyedSelectableListContent::new(
        name,
        snapshot,
        &mut rows(snapshot, base),
        ListBudget {
            max_items: 1024,
            max_display_items: 1024,
            max_utf8_bytes: 1_000_000,
            max_staging_units: 100_000,
        },
    )
    .expect("bounded registered Gallery list")
}
struct NavContent {
    host: ContentHost,
    state: SharedWorkbench,
    query: String,
    page: String,
    sections: Vec<super::navigation::Section>,
}
impl ContainerContent for NavContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        use super::navigation::Navigation;
        let page = self.state.borrow().selection().page.clone();
        if self.page != page {
            for section in &self.sections {
                let key = section.items.iter().find_map(|(candidate, _)| {
                    let super::Page::Workbench(index) = candidate else {
                        return None;
                    };
                    (workbench::entries()[*index].slug() == directory_page(&page))
                        .then_some(ListItemKey(*index as u64 + 1))
                });
                self.host
                    .set_keyed_list_selection(Navigation::list_id(section), key)
                    .expect("registered route selection");
            }
            self.page = page;
        }
        visit(&mut self.host);
        let query = self
            .host
            .get(FILTER)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap()
            .input()
            .text
            .clone();
        if query != self.query {
            // Only directory groups are replaced. The Search mount, IME state,
            // selection and focus remain owned by the same original Host.
            for section in &self.sections {
                self.host
                    .remove(Navigation::group_id(section))
                    .expect("mounted Gallery group");
            }
            self.sections = navigation_sections(&query);
            let selected =
                workbench::route(directory_page(&self.page)).expect("registered current page");
            for section in &self.sections {
                self.host
                    .insert(
                        super::navigation::LIST,
                        super::navigation::section_content(section, selected),
                    )
                    .expect("Gallery directory accepts original groups");
            }
            self.query = query;
        }
    }
    fn take_navigation_request(&mut self) -> Option<tekes_ui_app_host::navigation::Entry> {
        self.state.borrow_mut().pending.pop_front()
    }
}
fn navigation(state: SharedWorkbench) -> anyhow::Result<LocalContainer> {
    let sections = navigation_sections("");
    let selected = workbench::route(directory_page(&state.borrow().selection().page))
        .expect("registered current page");
    let inset = Spacing::Loose.value();
    let search = StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 0.,
        },
        children: vec![Content::new(
            FILTER,
            SearchFieldContent::new("Filter Gallery pages", ""),
        )],
        flexible_child: None,
    }
    .into_content_with_padding(
        ComponentId(11),
        tekes_ui_controls::layout::Insets::new(inset, inset, 0., inset),
    );
    let root = Content::new(
        ComponentId(10),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            children: vec![
                search,
                super::navigation::directory_content(&sections, selected),
            ],
            flexible_child: Some(super::navigation::SCROLL),
        },
    );
    let mut host =
        ContentHost::new(root).map_err(|error| anyhow::anyhow!("Gallery navigation: {error:?}"))?;
    host.set_action_events(true);
    let actions = state.clone();
    Ok(LocalContainer::from_source(
        "gallery-navigation",
        NavContent {
            host,
            state,
            query: String::new(),
            page: String::new(),
            sections,
        },
    )
    .content_padding(0.)
    .search_entry(FILTER)
    .on_action(move |_, records| {
        for record in records {
            let Some(section) = super::navigation::sections().iter().find(|section| {
                super::navigation::Navigation::list_id(section) == record.action.target
            }) else {
                continue;
            };
            if let Some(ContentActionPayload::ListIntent(intent)) = record.payload {
                let key = match intent.kind {
                    ListIntentKind::Activated => Some(intent.key),
                    ListIntentKind::SelectionChanged => intent.selected_key,
                };
                if let Some(index) = section.items.iter().find_map(|(page, _)| {
                    let super::Page::Workbench(index) = page else {
                        return None;
                    };
                    (key == Some(ListItemKey(*index as u64 + 1))).then_some(*index)
                }) {
                    actions
                        .borrow_mut()
                        .navigate(workbench::entries()[index].slug());
                }
            }
        }
    }))
}
fn chooser_snapshot(selection: &Selection) -> KeyedListSnapshot {
    let items = std::iter::once(KeyedListItem {
        key: ListItemKey(1),
        name: "Page guidance".into(),
    })
    .chain(
        specimens(&selection.page)
            .iter()
            .enumerate()
            .map(|(index, item)| KeyedListItem {
                key: ListItemKey(index as u64 + 2),
                name: item.label.into(),
            }),
    )
    .collect::<Vec<_>>();
    KeyedListSnapshot {
        dataset: ListDatasetKey(2),
        display_items: items.iter().map(|item| item.key).collect(),
        items,
    }
}
const DOCUMENTS: &[(&str, &str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/component_documents.rs"));
const SUPPLEMENTAL_DOCUMENTS: &[(&str, &str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/supplemental_documents.rs"));
const DOCUMENT_READER: ComponentId = ComponentId(21);
const DOCUMENT_SCROLL: ComponentId = ComponentId(22);
const DOCUMENT_NOTICE: ComponentId = ComponentId(23);
const DOCUMENT_HELP: &str =
    "⌘-click a link to read a bundled manual. External links are not supported.";

fn document_source(id: &str) -> Option<&'static str> {
    DOCUMENTS
        .iter()
        .find(|(key, _, _)| *key == id)
        .map(|(_, _, source)| *source)
}

fn document_body(selection: &Selection) -> String {
    match selection.document_id.as_deref() {
        Some(id) => match document_source(id) {
            Some(source) => source.into(),
            None => format!("Component manual not provided: {id}"),
        },
        None if workbench::apphost_quality::category_for_slug(&selection.page).is_some() => {
            let category = workbench::apphost_quality::category_for_slug(&selection.page).unwrap();
            let group = super::catalog::group(category);
            format!(
                "Page: {}\nOriginal cross-component state illustrations. Selecting an illustrated state is not live control acceptance.\nComponents: {}\nChoose a component to read its bundled manual.",
                group.name,
                group
                    .component_ids
                    .iter()
                    .map(|id| super::catalog::component(id).name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        None => format!(
            "Page: {}\n{}\n{}",
            selection.page,
            workbench::entries()
                .iter()
                .find(|entry| entry.slug() == selection.page)
                .map_or("", workbench::story_docs),
            if selection.specimen.is_some() {
                "This specimen has page guidance rather than a registered component manual."
            } else {
                "Select an available specimen to inspect its live instance and component manual."
            }
        ),
    }
}
fn document_key(selection: &Selection) -> String {
    selection
        .document_id
        .clone()
        .unwrap_or_else(|| format!("page:{}", selection.page))
}

fn document_scroll(key: &str, source: &str) -> Content {
    Content::new(
        DOCUMENT_SCROLL,
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                DOCUMENT_READER,
                RichContent::new(key, 1, RichDocument::from_markdown(source))
                    .expect("nonempty Gallery document identity"),
            ),
        ),
    )
}

/// Same document publications preserve the original text owner and selection.
/// Different documents retire the complete reader through the Host lifecycle.
fn publish_document(
    host: &mut ContentHost,
    key: &str,
    source: &str,
    pending_focus: &Cell<Option<u64>>,
) {
    let current = host
        .get(DOCUMENT_READER)
        .unwrap()
        .component::<RichContent>()
        .unwrap();
    let expected = current.version();
    if expected.identity() == key {
        if current.document().source() == source {
            return;
        }
        host.get_mut(DOCUMENT_READER)
            .unwrap()
            .component_mut::<RichContent>()
            .unwrap()
            .update(
                &expected,
                expected.revision() + 1,
                RichDocument::from_markdown(source),
            )
            .expect("synchronous Gallery document publication");
    } else {
        let replacement = document_scroll(key, source);
        let focused = host.focused() == Some(DOCUMENT_READER);
        host.remove(DOCUMENT_SCROLL)
            .expect("mounted Gallery document scroll");
        host.insert(ComponentId(20), replacement)
            .expect("Gallery document slot");
        pending_focus.set(if focused {
            host.semantic_mount(DOCUMENT_READER)
        } else {
            None
        });
    }
}

/// A replacement has no focusable geometry until the original Host arranges it.
/// Retain only that mount's one pending transfer, never a second focus owner.
fn finish_document_focus(host: &mut ContentHost, pending_focus: &Cell<Option<u64>>) {
    let Some(mount) = pending_focus.get() else {
        return;
    };
    if host.semantic_mount(DOCUMENT_READER) != Some(mount)
        || host.focused().is_some_and(|id| id != DOCUMENT_READER)
    {
        pending_focus.set(None);
        return;
    }
    if host
        .semantic_text_layout_snapshot(DOCUMENT_READER, None)
        .is_none()
    {
        return;
    }
    let restored =
        host.focus(Some(DOCUMENT_READER), false) || host.focused() == Some(DOCUMENT_READER);
    pending_focus.set(None);
    if !restored {
        document_notice(host, "Click the document to continue keyboard reading.");
    }
}
/// Resolve only bundled paths relative to the captured source manual. Never read
/// the filesystem or treat an arbitrary target as a command/URL to execute.
fn linked_document(document: &str, target: &str) -> Option<(&'static str, &'static str)> {
    let manuals = || DOCUMENTS.iter().chain(SUPPLEMENTAL_DOCUMENTS);
    let (_, path, _) = manuals().find(|(id, _, _)| *id == document)?;
    let joined = std::path::Path::new(path).parent()?.join(target);
    let mut resolved = std::path::PathBuf::new();
    for part in joined.components() {
        match part {
            std::path::Component::Normal(part) => resolved.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            _ => return None,
        }
    }
    manuals()
        .find(|(_, path, _)| std::path::Path::new(path) == resolved.as_path())
        .map(|(id, _, source)| (*id, *source))
}

fn document_notice(host: &mut ContentHost, message: &str) {
    if host
        .get(DOCUMENT_NOTICE)
        .unwrap()
        .component::<TextContent>()
        .unwrap()
        .text
        != message
    {
        host.get_mut(DOCUMENT_NOTICE)
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = message.into();
    }
}

/// Called only by the existing post-input action callback, even when the ordinary
/// action queue is empty. Rich links intentionally have their own owned queue.
fn consume_document_links(host: &mut ContentHost, pending_focus: &Cell<Option<u64>>) {
    let links = host
        .get_mut(DOCUMENT_READER)
        .unwrap()
        .component_mut::<RichContent>()
        .unwrap()
        .drain_link_actions();
    for link in links {
        if let Some((key, source)) = linked_document(&link.document, &link.target) {
            publish_document(host, key, source, pending_focus);
            document_notice(host, DOCUMENT_HELP);
        } else {
            document_notice(
                host,
                &format!(
                    "Link not opened: {} — only bundled manual links are supported.",
                    link.target
                ),
            );
        }
    }
}
/// Presentation data only: all values come from the selected mount's snapshot.
#[derive(Debug, PartialEq, Eq)]
struct InspectorGroup {
    title: &'static str,
    rows: Vec<(String, String)>,
}
impl InspectorGroup {
    fn new(title: &'static str) -> Self {
        Self {
            title,
            rows: Vec::new(),
        }
    }
    fn row(&mut self, label: impl Into<String>, value: impl ToString) {
        self.rows.push((label.into(), value.to_string()));
    }
}
fn inspector_body(
    selection: &Selection,
    snapshot: Option<InspectionSnapshot>,
) -> Vec<InspectorGroup> {
    let mut identity = InspectorGroup::new("Identity");
    identity.row("Page", &selection.page);
    identity.row("Specimen", selection.specimen.as_deref().unwrap_or("none"));
    identity.row(
        "Document",
        selection.document_id.as_deref().unwrap_or("page guidance"),
    );
    let Some(snapshot) = snapshot else {
        identity.row("Live snapshot", "Unavailable. Select a mounted specimen.");
        return vec![identity];
    };
    identity.row("Current instance", format!("{:?}", snapshot.id));
    identity.row("Mount", snapshot.mount);
    identity.row("Current semantics", format!("{:?}", snapshot.semantics));
    identity.row("Specimen focus", snapshot.focused);
    let mut properties = InspectorGroup::new("Properties");
    match snapshot.properties {
        Some(values) => {
            properties.row("Coverage", "Reported public properties only");
            for property in values {
                properties.row(property.name, property.value);
            }
        }
        None => properties.row("Coverage", "Not provided by this component."),
    }
    let mut layout = InspectorGroup::new("Layout");
    layout.row("Sizing", "Measured bounds, not declared sizing");
    // Keep the row schema stable across ordinary layout invalidation. A click
    // must still address the same text owner while the next paint is pending.
    let geometry = snapshot.geometry.as_ref();
    for (label, value) in [
        ("Frame", geometry.map(|g| format!("{:?}", g.frame))),
        (
            "Rectangular clip",
            geometry.map(|g| format!("{:?}", g.rectangular_clip)),
        ),
        (
            "Visible rectangle",
            geometry.map(|g| format!("{:?}", g.visible)),
        ),
        (
            "Additional curve geometry",
            geometry.map(|g| g.has_curve_geometry.to_string()),
        ),
    ] {
        layout.row(label, value.unwrap_or_else(|| "Unavailable".into()));
    }
    layout.row(
        "Current measured layout",
        if geometry.is_some() {
            "Available"
        } else {
            "Unavailable (not rendered or invalidated)."
        },
    );
    let mut tokens = InspectorGroup::new("Observed tokens");
    tokens.row(
        "Evidence",
        "Last paint (historical; not recomputed from current properties)",
    );
    tokens.row(
        "Limits",
        "Lookup calls do not prove every rendered pixel. Text and images are not sampled.",
    );
    tokens.row("Alias / override provenance", "Not provided");
    let mut quads = InspectorGroup::new("Emitted primitives");
    quads.row("Evidence", "Last paint, before GPU composition; quads only");
    match snapshot.last_paint {
        Some(paint) => {
            tokens.row("Theme", format!("{:?}", paint.theme));
            tokens.row("Includes owned children", paint.includes_owned_children);
            tokens.row("Lookup count", paint.tokens.lookups.len());
            for (family, name) in paint.tokens.lookups {
                tokens.row(format!("Lookup · {family}"), name);
            }
            tokens.row("Resolved color count", paint.tokens.colors.len());
            for color in paint.tokens.colors {
                tokens.row(
                    color.role,
                    format!("{:?}\nPalette: {:?}", color.value, color.palette),
                );
            }
            quads.row("Quad count", paint.quads.len());
            for (index, quad) in paint.quads.iter().enumerate() {
                quads.row(format!("Quad {}", index + 1), format!("{quad:#?}"));
            }
        }
        None => {
            tokens.row(
                "Recording",
                "Not recorded yet; wait for the specimen's normal paint.",
            );
            quads.row("Recording", "Not recorded yet");
        }
    }
    vec![identity, properties, layout, tokens, quads]
}

/// Shared key/value table owns aligned columns, padded rows and separators. The existing
/// ScrollContent owns overflow. Values use the shared selectable read-only text
/// owner; focusing them does not focus the specimen. IDs are panel-local.
fn inspector_sections(groups: &[InspectorGroup], mut next_id: u64) -> Vec<Content> {
    let mut sections = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let heading = Content::new(
            ComponentId(10_000 + index as u64 * 2),
            TextContent {
                text: group.title.into(),
                role: TextRole::UiHeading,
            },
        );
        let table_id = ComponentId(10_001 + index as u64 * 2);
        let mut rows = Vec::new();
        for (label, value) in &group.rows {
            rows.push((
                text(next_id, label),
                Content::new(
                    ComponentId(next_id + 1),
                    TextFieldContent::read_only_content(label, value, TextRole::UiBody),
                ),
            ));
            next_id += 2;
        }
        sections.push(heading);
        sections.push(Content::new(table_id, KeyValueTableContent::new(rows)));
    }
    sections
}
/// Both panels retain their own content and Gallery-owned chooser.
struct PanelContent {
    host: ContentHost,
    state: SharedWorkbench,
    selection: Option<Selection>,
    inspector: bool,
    inspector_groups: Vec<InspectorGroup>,
    inspector_target: Option<(ComponentId, u64)>,
    document_focus: Rc<Cell<Option<u64>>>,
}
impl PanelContent {
    fn publish(&mut self) {
        self.state.borrow_mut().sync_inspection();
        let selection = self.state.borrow().selection().clone();
        let selection_changed = self.selection.as_ref() != Some(&selection);
        if selection_changed {
            let snapshot = chooser_snapshot(&selection);
            self.host
                .replace_keyed_list(CHOOSER, &snapshot, &mut rows(&snapshot, 2000), None)
                .expect("registered specimen chooser");
            let key = selection
                .specimen
                .as_deref()
                .and_then(|key| {
                    specimens(&selection.page)
                        .iter()
                        .position(|item| item.key == key)
                })
                .map_or(1, |index| index as u64 + 2);
            self.host
                .set_keyed_list_selection(CHOOSER, Some(ListItemKey(key)))
                .expect("registered specimen selection");
        }
        if self.inspector {
            let snapshot = self.state.borrow().selected_snapshot();
            let target = snapshot
                .as_ref()
                .map(|snapshot| (snapshot.id, snapshot.mount));
            let groups = inspector_body(&selection, snapshot);
            if groups != self.inspector_groups {
                if selection_changed || target != self.inspector_target {
                    self.host
                        .remove(DOCUMENT_READER)
                        .expect("mounted Inspector groups");
                    self.host
                        .insert(
                            DOCUMENT_SCROLL,
                            stack(DOCUMENT_READER.0, inspector_sections(&groups, 10_010)),
                        )
                        .expect("Inspector scroll content");
                } else {
                    // AppHost cancels other panels before a click, invalidating
                    // specimen geometry until its next paint. Keep unchanged
                    // groups mounted through that transition, including the
                    // value receiving this click and its native/AX text owner.
                    let next_id = self
                        .host
                        .get(DOCUMENT_READER)
                        .unwrap()
                        .component::<StackContent>()
                        .unwrap()
                        .children
                        .iter()
                        .filter_map(|node| node.component::<KeyValueTableContent>())
                        .flat_map(|table| table.cells().iter().map(|cell| cell.id().0))
                        .max()
                        .map_or(10_010, |id| id + 1);
                    for (index, section) in
                        inspector_sections(&groups, next_id).into_iter().enumerate()
                    {
                        let group = index / 2;
                        if self.inspector_groups.get(group) == groups.get(group) {
                            continue;
                        }
                        if self.inspector_groups[group]
                            .rows
                            .iter()
                            .map(|(label, _)| label)
                            .eq(groups[group].rows.iter().map(|(label, _)| label))
                        {
                            if index % 2 == 1 {
                                let values: Vec<_> = self
                                    .host
                                    .get(section.id())
                                    .unwrap()
                                    .component::<KeyValueTableContent>()
                                    .unwrap()
                                    .cells()
                                    .chunks_exact(2)
                                    .zip(&groups[group].rows)
                                    .map(|(cells, (_, value))| (cells[1].id(), value.clone()))
                                    .collect();
                                for (id, value) in values {
                                    if self.host.text_surface(id).unwrap().input.text == value {
                                        continue;
                                    }
                                    let field = self
                                        .host
                                        .get_mut(id)
                                        .unwrap()
                                        .component_mut::<TextFieldContent>()
                                        .unwrap();
                                    let baseline = field.surface.input.external_baseline();
                                    let revision = tekes_ui_controls::DraftRevision {
                                        identity: "gallery-inspector-value".into(),
                                        version: field
                                            .surface
                                            .input
                                            .external_value()
                                            .map_or(1, |v| v.version + 1),
                                        payload: value,
                                    };
                                    let result = field.surface.apply_external_value(
                                        &baseline,
                                        &revision,
                                        tekes_ui_controls::ExternalSelection::Reset,
                                        tekes_ui_controls::ExternalUndo::Reset,
                                    );
                                    assert!(
                                        matches!(
                                            result,
                                            tekes_ui_controls::ExternalValueResult::Applied { .. }
                                        ),
                                        "synchronous Inspector value publication: {result:?}"
                                    );
                                }
                            }
                            continue;
                        }
                        self.host
                            .remove(section.id())
                            .expect("mounted Inspector section");
                        self.host
                            .insert(DOCUMENT_READER, section)
                            .expect("Inspector section");
                    }
                    let order: Vec<_> = (0..groups.len() * 2)
                        .map(|index| ComponentId(10_000 + index as u64))
                        .collect();
                    self.host
                        .reorder(DOCUMENT_READER, &order)
                        .expect("Inspector section order");
                }
                self.inspector_groups = groups;
            }
            self.inspector_target = target;
        } else {
            // Only an explicit chooser activation requests a return after link browsing.
            // Ordinary reads and select(same) keep the linked reader intact.
            let return_requested =
                std::mem::take(&mut self.state.borrow_mut().document_return_requested);
            if selection_changed || return_requested {
                publish_document(
                    &mut self.host,
                    &document_key(&selection),
                    &document_body(&selection),
                    &self.document_focus,
                );
                document_notice(&mut self.host, DOCUMENT_HELP);
            }
        }
        self.selection = Some(selection);
    }
}
impl ContainerContent for PanelContent {
    fn prepare_for_render(&mut self) {
        if self.inspector {
            self.publish();
        }
    }
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        if !self.inspector {
            self.publish();
        }
        visit(&mut self.host);
        if !self.inspector {
            finish_document_focus(&mut self.host, &self.document_focus);
        }
    }
}
fn consume_chooser_actions(state: &SharedWorkbench, records: Vec<ContentActionRecord>) {
    for record in records {
        if record.action.target != CHOOSER {
            continue;
        }
        if let Some(ContentActionPayload::ListIntent(intent)) = record.payload {
            let key = match intent.kind {
                ListIntentKind::Activated => Some(intent.key),
                ListIntentKind::SelectionChanged => intent.selected_key,
            };
            if let Some(key) = key {
                let mut state = state.borrow_mut();
                let specimen = key
                    .0
                    .checked_sub(2)
                    .and_then(|index| specimens(&state.selection().page).get(index as usize))
                    .map(|item| item.key);
                state.select(specimen);
                if intent.kind == ListIntentKind::Activated {
                    state.document_return_requested = true;
                }
            }
        }
    }
}

fn panel_source(state: SharedWorkbench, inspector: bool) -> anyhow::Result<PanelContent> {
    let snapshot = chooser_snapshot(state.borrow().selection());
    let mut host = ContentHost::new(stack(
        20,
        vec![
            Content::new(CHOOSER, list("Inspect specimen", &snapshot, 2000)),
            if inspector {
                Content::new(
                    DOCUMENT_SCROLL,
                    ScrollContent::new(ScrollArea::default(), stack(DOCUMENT_READER.0, Vec::new()))
                        .scrollbar(true),
                )
            } else {
                let selection = state.borrow().selection().clone();
                document_scroll(&document_key(&selection), &document_body(&selection))
            },
        ],
    ))
    .map_err(|error| anyhow::anyhow!("Gallery panel: {error:?}"))?;
    if !inspector {
        host.insert(
            ComponentId(20),
            Content::new(DOCUMENT_NOTICE, TextContent::new(DOCUMENT_HELP)),
        )
        .expect("Gallery document help");
        host.reorder(
            ComponentId(20),
            &[CHOOSER, DOCUMENT_NOTICE, DOCUMENT_SCROLL],
        )
        .expect("Gallery document order");
    }
    host.set_action_events(true);
    host.get_mut(ComponentId(20))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .flexible_child = Some(ComponentId(22));
    Ok(PanelContent {
        host,
        state,
        selection: None,
        inspector,
        inspector_groups: Vec::new(),
        inspector_target: None,
        document_focus: Rc::new(Cell::new(None)),
    })
}
fn panel(
    id: &str,
    title: &str,
    state: SharedWorkbench,
    inspector: bool,
) -> anyhow::Result<LocalContainer> {
    let source = panel_source(state.clone(), inspector)?;
    let document_focus = source.document_focus.clone();
    Ok(LocalContainer::from_source(id, source)
        .title(title)
        .on_action(move |host, records| {
            if !inspector {
                consume_document_links(host, &document_focus);
            }
            consume_chooser_actions(&state, records);
        }))
}
fn assemble_with_session_path() -> anyhow::Result<(AppWindow, std::path::PathBuf)> {
    let page = std::env::var("TEKESUI_GALLERY_PAGE").unwrap_or_else(|_| SEARCH_ROUTE.into());
    let palette = Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)));
    let host = page_host(&page, &palette)?;
    let settings = super::apphost::settings_owner()?;
    let root = std::env::var_os("TEKESUI_APPHOST_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("tekesui-gallery-workbench"));
    let history_id = format!(
        "workbench-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let session_path = root.join(&history_id);
    let state = Rc::new(RefCell::new(WorkbenchState {
        settings: Some(settings.clone()),
        window_requests: 0,
        palette: palette.clone(),
        current: 0,
        visits: vec![Selection::new(&page, None)],
        pages: HashMap::from([(page, Rc::new(RefCell::new(host)))]),
        pending: VecDeque::new(),
        inspected: None,
        document_return_requested: false,
        session_dirs: vec![super::apphost::SessionTempDir::new(session_path.clone())],
    }));
    let initial = state.borrow().entry(0);
    let commands = state.clone();
    let focus = LocalContainer::from_source("gallery-focus", FocusContent(state.clone()))
        .title("Gallery")
        .on_command(move |_, command| commands.borrow_mut().restore(command));
    let presentation = PresentationOptions::new("gallery-focus")
        .nav_bar("gallery-navigation")
        .navigation_toolbar("gallery-navigation")
        .navigation_targets(vec!["gallery-focus".into()])
        .others(OthersOptions::new(vec![
            "gallery-documents".into(),
            "gallery-inspector".into(),
        ]));
    let app = AppWindow::from_containers(
        vec![
            navigation(state.clone())?.into(),
            focus.into(),
            panel("gallery-documents", "Documents", state.clone(), false)?.into(),
            panel("gallery-inspector", "Inspector", state, true)?.into(),
        ],
        presentation,
    )?
    .navigation(NavigationOptions {
        directory: session_path.clone(),
        history_id,
        initial_entries: vec![initial],
        view_command: Some(|entry| {
            anyhow::ensure!(known_page(&entry.route), "unknown Gallery route");
            entry
                .params
                .get("visit")
                .and_then(serde_json::Value::as_u64)
                .and_then(|visit| COMMAND.checked_add(visit))
                .ok_or_else(|| anyhow::anyhow!("invalid Gallery history visit"))
        }),
    })
    .palette_source(palette)
    .shared_settings(settings);
    Ok((app, session_path))
}
/// Default to AppHost; retain the explicit selector and its original arguments.
pub fn apphost_arguments(args: &[String]) -> Option<Vec<String>> {
    match args {
        [_] => Some(args.to_vec()),
        [_, selector, ..] if selector == "--apphost" => {
            Some(args[..1].iter().chain(&args[2..]).cloned().collect())
        }
        _ => None,
    }
}
pub fn run(args: &[String]) -> anyhow::Result<()> {
    let (app, session_path) = assemble_with_session_path()?;
    let _session_dir = super::apphost::SessionTempDir::new(session_path);
    app.run_with_args(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::composition::ContentEvent;

    #[test]
    fn apphost_all_registered_routes_mount_real_page_content() {
        let palette = Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)));
        assert_eq!(workbench::entries().len(), 70);
        for entry in workbench::entries() {
            let page = page_host(entry.slug(), &palette).unwrap();
            let notice = page
                .get(ComponentId(202))
                .and_then(|node| node.component::<TextContent>());
            assert!(
                !notice.is_some_and(|copy| copy.text.starts_with("This route has not migrated")),
                "{}",
                entry.slug()
            );
        }
    }
    #[test]
    fn apphost_launcher_navigation_and_repaint_do_not_enqueue_windows() {
        use tekes_ui_controls::composition::AccessibilityAction;
        let state = state("apphost");
        let mut text = super::super::text::TextEngine::new();
        let mut focus = focus_container(state.clone());
        for _ in 0..2 {
            focus.with_host(&mut |host| render_text_host(host, &mut text));
        }
        assert_eq!(state.borrow().window_requests, 0);
        focus.with_host(&mut |host| {
            assert!(
                host.accessibility_action(ComponentId(124020), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        assert_eq!(state.borrow().window_requests, 1);
        for _ in 0..2 {
            focus.with_host(&mut |host| render_text_host(host, &mut text));
        }
        assert_eq!(state.borrow().window_requests, 1);
        assert_eq!(state.borrow().visits.len(), 1);
    }
    #[test]
    fn apphost_workbench_consumes_only_its_entry_argument() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };
        let launch = args(&["/retained/program/tekes-ui-gallery", "--apphost"]);
        assert_eq!(apphost_arguments(&launch), Some(vec![launch[0].clone()]));
        for tail in [
            vec!["--window-acceptance"],
            vec!["--modules", "unknown"],
            vec!["--worker", "unknown", "socket"],
            vec!["unexpected"],
        ] {
            let mut launch = args(&["gallery", "--apphost"]);
            launch.extend(tail.iter().map(|value| (*value).to_owned()));
            let mut expected = args(&["gallery"]);
            expected.extend(tail.into_iter().map(str::to_owned));
            assert_eq!(apphost_arguments(&launch), Some(expected));
        }
        let default_launch = args(&["gallery"]);
        assert_eq!(
            apphost_arguments(&default_launch),
            Some(default_launch.clone())
        );
        for other in [
            vec!["gallery", "--legacy"],
            vec!["gallery", "--legacy", "--snapshot", "output"],
            vec!["gallery", "--snapshot", "output"],
            vec!["gallery", "--worker", "module", "socket"],
        ] {
            assert!(apphost_arguments(&args(&other)).is_none());
        }
    }
    fn state(page: &str) -> SharedWorkbench {
        let palette = Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)));
        Rc::new(RefCell::new(WorkbenchState {
            settings: None,
            window_requests: 0,
            palette: palette.clone(),
            current: 0,
            visits: vec![Selection::new(page, None)],
            pages: HashMap::from([(
                page.into(),
                Rc::new(RefCell::new(page_host(page, &palette).unwrap())),
            )]),
            pending: VecDeque::new(),
            inspected: None,
            document_return_requested: false,
            session_dirs: Vec::new(),
        }))
    }
    fn render_text_host(host: &mut ContentHost, text: &mut super::super::text::TextEngine) {
        let frame = tekes_ui_scene::Rect::new(0., 0., 740., 900.);
        host.render(
            &mut tekes_ui_scene::Scene::new(),
            text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
    }
    fn render_panel(panel: &mut PanelContent, text: &mut super::super::text::TextEngine) {
        // Same production source and hook/Host sequence. AppHost's own tests
        // separately exercise LocalContainer::render wiring and query ordering.
        panel.prepare_for_render();
        panel.with_host(&mut |host| render_text_host(host, text));
    }
    fn focus_container(state: SharedWorkbench) -> LocalContainer {
        LocalContainer::from_source("test-focus", FocusContent(state))
    }
    fn publish(state: &SharedWorkbench, page: &str) {
        state.borrow_mut().navigate(page);
        let entry = state.borrow_mut().pending.pop_front().unwrap();
        let visit = entry.params["visit"].as_u64().unwrap();
        // LocalContainer invokes its command while borrowing the previous host.
        // The shell must not retain a borrow of the application model here.
        focus_container(state.clone()).with_host(&mut |_| {
            state.borrow_mut().restore(COMMAND + visit).unwrap();
        });
    }
    #[test]
    fn apphost_workbench_history_retains_each_visit_and_live_input() {
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let mut focus = focus_container(state.clone());
        let mut text = super::super::text::TextEngine::new();
        focus.with_host(&mut |host| {
            // Formal focus/reveal admission needs the real laid-out text host.
            render_text_host(host, &mut text);
            assert!(host.focus(Some(ComponentId(100)), false));
            render_text_host(host, &mut text);
            assert_eq!(host.focused(), Some(ComponentId(100)));
            host.event_with_text(
                &ContentEvent::Commit {
                    text: " edited".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
            assert!(
                host.text_surface(ComponentId(100))
                    .unwrap()
                    .input
                    .text
                    .contains("edited")
            );
        });
        assert_eq!(state.borrow().selection().specimen.as_deref(), Some("name"));
        publish(&state, SEARCH_ROUTE);
        assert_eq!(
            state.borrow().selection().document_id.as_deref(),
            Some("tui.search-field")
        );
        publish(&state, FORM_ROUTE);
        state.borrow_mut().select(Some("published"));
        assert_eq!(
            state.borrow().selection().document_id.as_deref(),
            Some("tui.checkbox")
        );
        state.borrow_mut().restore(COMMAND).unwrap();
        assert_eq!(state.borrow().selection().specimen.as_deref(), Some("name"));
        focus.with_host(&mut |host| {
            assert!(
                host.text_surface(ComponentId(100))
                    .unwrap()
                    .input
                    .text
                    .contains("edited")
            )
        });
        state.borrow_mut().restore(COMMAND + 2).unwrap();
        assert_eq!(
            state.borrow().selection().specimen.as_deref(),
            Some("published")
        );
        // Choosing an inspection target never inserts a page visit.
        assert_eq!(state.borrow().visits.len(), 3);
        assert!(state.borrow().pending.is_empty());
    }
    #[test]
    fn apphost_quality_navigation_retains_state_and_consistency_directory() {
        use tekes_ui_controls::composition::AccessibilityAction;
        let state = state("lab-consistency");
        let mut focus = focus_container(state.clone());
        let mut directory = navigation(state.clone()).unwrap();
        let mut text = super::super::text::TextEngine::new();
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(
                host.accessibility_action(ComponentId(100101), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        let entry = state.borrow_mut().pending.pop_front().unwrap();
        assert_eq!(entry.route, "components-actions");
        let visit = entry.params["visit"].as_u64().unwrap();
        focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND + visit).unwrap());
        let component = super::super::catalog::component(
            &super::super::catalog::group("actions").component_ids[0],
        );
        let chosen = &component.states[1];
        state.borrow_mut().select(Some(component.id.as_str()));
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let node = host
                .semantics()
                .into_iter()
                .find(|node| node.id == workbench::apphost_quality::PREVIEW)
                .unwrap();
            let part = node
                .parts
                .iter()
                .find(|part| part.semantics.name == format!("{}: {chosen}", component.name))
                .unwrap()
                .index;
            assert!(
                host.accessibility_action(node.id, Some(part), AccessibilityAction::Press)
                    .consumed
            );
        });
        let cached = state.borrow().pages["components-actions"].clone();
        focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND).unwrap());
        assert_eq!(state.borrow().selection().page, "lab-consistency");
        focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND + visit).unwrap());
        assert!(Rc::ptr_eq(
            &cached,
            &state.borrow().pages["components-actions"]
        ));
        focus.with_host(&mut |host| render_text_host(host, &mut text));
        let snapshot = state.borrow().selected_snapshot().unwrap();
        assert!(
            snapshot
                .properties
                .unwrap()
                .iter()
                .any(|property| property.name == component.id && property.value == *chosen)
        );
        directory.with_host(&mut |host| {
            render_text_host(host, &mut text);
            // The full directory is taller than this viewport. Offscreen rows
            // deliberately have no AX parts; selection belongs to the real list.
            let index = workbench::entries()
                .iter()
                .position(|entry| entry.slug() == "lab-consistency")
                .unwrap();
            let section = navigation_sections("")
                .into_iter()
                .find(|section| {
                    section
                        .items
                        .iter()
                        .any(|(page, _)| *page == super::super::Page::Workbench(index))
                })
                .unwrap();
            let list = host
                .get(super::super::navigation::Navigation::list_id(&section))
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap();
            assert_eq!(list.selected_key(), Some(ListItemKey(index as u64 + 1)));
        });
        assert_eq!(
            navigation_sections("")
                .iter()
                .map(|section| section.items.len())
                .sum::<usize>(),
            70
        );
        assert_eq!(state.borrow().visits.len(), 2);
    }
    #[test]
    fn apphost_workbench_selection_rejects_retired_specimens_and_visits() {
        let state = state(FORM_ROUTE);
        assert_eq!(state.borrow().selection().specimen, None);
        state.borrow_mut().select(Some("retired-field"));
        assert_eq!(state.borrow().selection().document_id, None);
        let before = state.borrow().selection().clone();
        assert!(state.borrow_mut().restore(COMMAND + 99).is_err());
        assert_eq!(*state.borrow().selection(), before);
        assert!(
            page_host(
                "retired-route",
                &Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)))
            )
            .is_err()
        );
        for route in [SEARCH_ROUTE, FORM_ROUTE] {
            for specimen in specimens(route) {
                assert!(
                    workbench::entries()
                        .iter()
                        .any(|entry| Some(entry.component()) == specimen.document_id)
                );
            }
        }
    }
    #[test]
    fn apphost_workbench_filter_retains_registry_identity() {
        let all = navigation_sections("");
        assert_eq!(
            all.iter().map(|section| section.items.len()).sum::<usize>(),
            workbench::entries().len()
        );
        let filtered = navigation_sections("SEARCH-FIELD");
        assert!(!filtered.is_empty());
        for section in filtered {
            let original = all
                .iter()
                .find(|original| original.title == section.title)
                .unwrap();
            assert_eq!(
                super::super::navigation::Navigation::list_id(&section),
                super::super::navigation::Navigation::list_id(original)
            );
            for (page, _) in section.items {
                assert!(
                    original
                        .items
                        .iter()
                        .any(|(candidate, _)| *candidate == page)
                );
            }
        }
        assert!(navigation_sections("no-such-gallery-component-291").is_empty());
    }
    #[test]
    fn apphost_workbench_navigation_reuses_group_headers_and_keeps_search_mount() {
        use super::super::navigation::{LIST, Navigation};
        let state = state(SEARCH_ROUTE);
        let mut nav = navigation(state.clone()).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let frame = tekes_ui_scene::Rect::new(0., 60., 300., 700.);
        let mut search_mount = 0;
        nav.with_host(&mut |host| {
            host.render(
                &mut tekes_ui_scene::Scene::new(),
                &mut text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
            let search = host.get(FILTER).unwrap().frame();
            assert_eq!(search.min_x() - frame.min_x(), Spacing::Loose.value());
            assert_eq!(frame.max_x() - search.max_x(), Spacing::Loose.value());
            assert_eq!(search.min_y() - frame.min_y(), Spacing::Loose.value());
            let rows = host.get(LIST).unwrap().frame();
            assert_eq!(rows.min_x(), search.min_x());
            assert_eq!(rows.max_x(), search.max_x());
            assert!(rows.min_y() > search.max_y());
            for section in super::super::navigation::sections() {
                let list = host
                    .get(Navigation::list_id(section))
                    .unwrap()
                    .component::<KeyedSelectableListContent>()
                    .unwrap();
                assert_eq!(list.model().snapshot().items.len(), section.items.len());
                for (item, (page, _)) in list.model().snapshot().items.iter().zip(&section.items) {
                    assert_eq!(item.name, page.name());
                }
            }
            search_mount = host.get(FILTER).unwrap().mount_identity();
            assert!(host.focus(Some(FILTER), true));
            host.event_with_text(
                &ContentEvent::Commit {
                    text: "SearchField".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
        });
        nav.with_host(&mut |host| {
            assert_eq!(host.get(FILTER).unwrap().mount_identity(), search_mount);
            assert_eq!(host.focused(), Some(FILTER));
            let displayed = navigation_sections("SearchField");
            assert_eq!(
                displayed
                    .iter()
                    .map(|section| section.items.len())
                    .sum::<usize>(),
                1
            );
            let section = &displayed[0];
            assert!(host.get(Navigation::group_id(section)).is_some());
            assert!(
                host.get(Navigation::group_id(
                    &super::super::navigation::sections()[0]
                ))
                .is_none()
            );
        });
        assert!(
            state.borrow().pending.is_empty(),
            "filtering must not push navigation history"
        );
    }
    #[test]
    fn apphost_workbench_manual_and_snapshot_follow_exact_instance() {
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        state.borrow_mut().sync_inspection();
        let name = state.borrow().selected_snapshot().unwrap();
        let field_manual = document_body(state.borrow().selection());
        assert!(field_manual.contains("TextField"));
        assert_eq!(
            name.properties
                .as_ref()
                .unwrap()
                .iter()
                .find(|property| property.name == "text")
                .unwrap()
                .value,
            "Gallery"
        );
        state.borrow_mut().select(Some("owner"));
        state.borrow_mut().sync_inspection();
        let owner = state.borrow().selected_snapshot().unwrap();
        assert_eq!(document_body(state.borrow().selection()), field_manual);
        assert_ne!(name.id, owner.id);
        assert_ne!(name.mount, owner.mount);
        assert_eq!(
            owner
                .properties
                .as_ref()
                .unwrap()
                .iter()
                .find(|property| property.name == "text")
                .unwrap()
                .value,
            "TekesUI team"
        );
        assert_eq!(
            state.borrow().inspected,
            Some((FORM_ROUTE.into(), ComponentId(102), owner.mount))
        );
        state.borrow_mut().select(Some("published"));
        state.borrow_mut().sync_inspection();
        assert_eq!(
            state.borrow().selection().document_id.as_deref(),
            Some("tui.checkbox")
        );
        let checkbox_manual = document_body(state.borrow().selection());
        assert!(checkbox_manual.contains("Checkbox"));
        assert_ne!(checkbox_manual, field_manual);
        assert_eq!(
            state.borrow().selected_snapshot().unwrap().id,
            ComponentId(101)
        );
        state.borrow_mut().select(None);
        state.borrow_mut().sync_inspection();
        assert!(state.borrow().selected_snapshot().is_none());
        assert!(state.borrow().inspected.is_none());
        assert!(document_source("retired-component").is_none());
    }
    #[test]
    fn apphost_workbench_inspector_replaces_retired_rows_and_keeps_unchanged_mount() {
        fn rows(host: &ContentHost) -> Vec<(String, String)> {
            host.get(DOCUMENT_READER)
                .unwrap()
                .component::<StackContent>()
                .unwrap()
                .children
                .iter()
                .filter_map(|section| section.component::<KeyValueTableContent>())
                .flat_map(|table| {
                    table.cells().chunks_exact(2).map(|pair| {
                        (
                            pair[0].component::<TextContent>().unwrap().text.clone(),
                            pair[1]
                                .component::<TextFieldContent>()
                                .unwrap()
                                .surface
                                .input
                                .text
                                .clone(),
                        )
                    })
                })
                .collect()
        }
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let mut inspector = panel_source(state.clone(), true).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let mut reader_mount = None;
        let mut scroll_mount = None;
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            let rows = rows(host);
            assert!(rows.contains(&("text".into(), "Gallery".into())));
            assert!(rows.iter().any(|(key, value)| key == "Current measured layout" && value.starts_with("Unavailable")));
            assert!(rows.iter().any(|(key, value)| key == "Recording" && value.starts_with("Not recorded yet")));
            reader_mount = host.semantic_mount(DOCUMENT_READER);
            scroll_mount = host.semantic_mount(DOCUMENT_SCROLL);
        });
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(DOCUMENT_READER), reader_mount);
        });
        state.borrow_mut().select(Some("owner"));
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            assert_ne!(host.semantic_mount(DOCUMENT_READER), reader_mount);
            assert_eq!(host.semantic_mount(DOCUMENT_SCROLL), scroll_mount);
            let rows = rows(host);
            assert!(rows.contains(&("text".into(), "TekesUI team".into())));
            assert!(!rows.iter().any(|(_, value)| value == "Gallery"));
        });
        state.borrow_mut().select(Some("published"));
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            assert!(!rows(host).iter().any(|(key, _)| key == "text"));
        });
        // Area is a multiline TextFieldContent and has its property hook.
        // The actual ComboBoxContent mount has no hook, so it exercises the
        // unavailable-properties boundary without inventing a snapshot.
        publish(&state, COMBO_ROUTE);
        {
            let state = state.borrow();
            assert_eq!(
                state.selection().specimen.as_deref(),
                Some("original-combo")
            );
            let snapshot = state
                .selected_snapshot()
                .expect("mounted ComboBox specimen");
            assert_eq!(snapshot.id, ComponentId(27601));
            assert_eq!(
                Some(snapshot.mount),
                state.pages[COMBO_ROUTE]
                    .borrow()
                    .semantic_mount(snapshot.id),
            );
            assert!(snapshot.properties.is_none());
        }
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            let rows = rows(host);
            assert!(rows.contains(&("Coverage".into(), "Not provided by this component.".into())));
            assert!(!rows.iter().any(|(key, _)| key == "text"));
        });
        state.borrow_mut().select(None);
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            let rows = rows(host);
            assert!(
                rows.iter()
                    .any(|(key, value)| key == "Live snapshot" && value.starts_with("Unavailable"))
            );
            assert!(
                !rows
                    .iter()
                    .any(|(key, _)| key == "Coverage" || key == "Theme" || key == "Frame")
            );
            assert_eq!(host.semantic_mount(DOCUMENT_SCROLL), scroll_mount);
        });
    }
    struct InspectorClipboard(RefCell<String>);
    impl tekes_ui_module_runtime::ContentClipboard for InspectorClipboard {
        fn clipboard_text(&self) -> Option<String> {
            Some(self.0.borrow().clone())
        }
        fn set_clipboard_text(&self, value: &str) {
            *self.0.borrow_mut() = value.into();
        }
    }
    fn inspector_value_id(host: &ContentHost, label: &str) -> ComponentId {
        host.get(DOCUMENT_READER)
            .unwrap()
            .component::<StackContent>()
            .unwrap()
            .children
            .iter()
            .filter_map(|section| section.component::<KeyValueTableContent>())
            .flat_map(|table| table.cells().chunks_exact(2))
            .find(|pair| pair[0].component::<TextContent>().unwrap().text == label)
            .unwrap()[1]
            .id()
    }
    #[test]
    fn apphost_workbench_inspector_values_select_copy_and_reject_edits() {
        use tekes_ui_controls::composition::AccessibilityAction;
        use tekes_ui_module_runtime::{ContentClipboard, dispatch_content_window_event_with_text};
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let mut inspector = panel_source(state.clone(), true).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let clipboard = InspectorClipboard(RefCell::new(String::new()));
        focus_container(state.clone()).with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.focus(Some(ComponentId(100)), false));
            render_text_host(host, &mut text);
        });
        render_panel(&mut inspector, &mut text);
        // Actual AppHost mouse-down routing cancels every other local panel
        // before delivering the click. That invalidates specimen layout.
        focus_container(state.clone()).with_host(&mut |host| {
            host.cancel();
            host.focus(None, false);
        });
        assert!(
            state
                .borrow()
                .selected_snapshot()
                .unwrap()
                .geometry
                .is_none()
        );
        let mut value_mount = None;
        inspector.with_host(&mut |host| {
            let value = inspector_value_id(host, "Specimen");
            let point = host.get(value).unwrap().frame().center();
            host.event_with_text(
                &tekes_ui_controls::composition::ContentEvent::PointerDown(point),
                &mut text,
            )
            .unwrap();
            host.event_with_text(
                &tekes_ui_controls::composition::ContentEvent::PointerUp(point),
                &mut text,
            )
            .unwrap();
            assert_eq!(host.focused(), Some(value));
            value_mount = host.semantic_mount(value);
            host.accessibility_action(value, None, AccessibilityAction::SetSelection(0..4));
            assert_eq!(
                host.text_surface(value).unwrap().input.selected_text(),
                "name"
            );
            host.accessibility_action(
                value,
                None,
                AccessibilityAction::ReplaceSelectedText("changed".into()),
            );
            host.accessibility_action(value, None, AccessibilityAction::SetValue("changed".into()));
            assert_eq!(host.text_surface(value).unwrap().input.text, "name");
            assert!(host.text_surface(value).unwrap().input.read_only);
        });
        // The next ordinary Focus paint restores geometry. Publishing that
        // Layout group must not retire the clicked Identity text owner.
        focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
        assert!(
            state
                .borrow()
                .selected_snapshot()
                .unwrap()
                .geometry
                .is_some()
        );
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            let value = inspector_value_id(host, "Specimen");
            assert_eq!(host.semantic_mount(value), value_mount);
            assert_eq!(host.focused(), Some(value));
            dispatch_content_window_event_with_text(
                host,
                &tekes_ui_platform_macos::Event::TextCommand(
                    tekes_ui_platform_macos::TextEditCommand::Copy,
                ),
                &clipboard,
                &mut text,
            )
            .unwrap();
            assert_eq!(clipboard.clipboard_text().as_deref(), Some("name"));
            assert_eq!(
                host.text_surface(inspector_value_id(host, "Specimen"))
                    .unwrap()
                    .input
                    .selected_text(),
                "name"
            );
        });
        state.borrow_mut().select(Some("owner"));
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            let input = &host
                .text_surface(inspector_value_id(host, "Specimen"))
                .unwrap()
                .input;
            assert_eq!(input.text, "owner");
            assert!(input.selected_text().is_empty());
        });
    }
    #[test]
    fn apphost_workbench_inspector_layout_value_survives_invalidation_and_copies_current_frame() {
        use tekes_ui_controls::composition::{AccessibilityAction, ContentEvent};
        use tekes_ui_module_runtime::{ContentClipboard, dispatch_content_window_event_with_text};
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let mut inspector = panel_source(state.clone(), true).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let clipboard = InspectorClipboard(RefCell::new(String::new()));
        focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
        let current_frame = format!(
            "{:?}",
            state
                .borrow()
                .selected_snapshot()
                .unwrap()
                .geometry
                .unwrap()
                .frame
        );
        render_panel(&mut inspector, &mut text);
        let mut frame_id = ComponentId(0);
        inspector.with_host(&mut |host| {
            frame_id = inspector_value_id(host, "Frame");
            let y = host.get(frame_id).unwrap().frame().min_y();
            host.get_mut(DOCUMENT_SCROLL)
                .unwrap()
                .component_mut::<ScrollContent>()
                .unwrap()
                .scroll
                .scroll_by(0., y - 300.);
        });
        render_panel(&mut inspector, &mut text);
        let mut frame_mount = None;
        let mut frame_point = tekes_ui_scene::Point::new(0., 0.);
        inspector.with_host(&mut |host| {
            let frame = host.get(frame_id).unwrap().frame();
            frame_point = tekes_ui_scene::Point::new(frame.min_x() + 8., frame.min_y() + 8.);
            assert!(
                host.get(frame_id)
                    .unwrap()
                    .visible()
                    .unwrap()
                    .contains(frame_point)
            );
            frame_mount = host.semantic_mount(frame_id);
        });
        // Real mouse-down ordering: another panel is cancelled before the
        // Inspector receives input. There is no prepare/render between them.
        focus_container(state.clone()).with_host(&mut |host| {
            host.cancel();
            host.focus(None, false);
        });
        assert!(
            state
                .borrow()
                .selected_snapshot()
                .unwrap()
                .geometry
                .is_none()
        );
        inspector.with_host(&mut |host| {
            assert_eq!(
                host.text_surface(frame_id).unwrap().input.text,
                current_frame
            );
            host.event_with_text(&ContentEvent::PointerDown(frame_point), &mut text)
                .unwrap();
            host.event_with_text(&ContentEvent::PointerUp(frame_point), &mut text)
                .unwrap();
            assert_eq!(host.focused(), Some(frame_id));
            let len = host.text_surface(frame_id).unwrap().input.len();
            host.accessibility_action(frame_id, None, AccessibilityAction::SetSelection(0..len));
        });
        focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(frame_id), frame_mount);
            assert_eq!(host.focused(), Some(frame_id));
            assert_eq!(
                host.text_surface(frame_id).unwrap().input.selected_text(),
                current_frame
            );
        });
        // If the source really remains invalid at a normal Inspector draw,
        // publish Unavailable honestly and reset only that changed value.
        focus_container(state.clone()).with_host(&mut |host| host.cancel());
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(frame_id), frame_mount);
            assert_eq!(host.focused(), Some(frame_id));
            assert_eq!(
                host.text_surface(frame_id).unwrap().input.text,
                "Unavailable"
            );
            assert!(
                host.text_surface(frame_id)
                    .unwrap()
                    .input
                    .selected_text()
                    .is_empty()
            );
            host.accessibility_action(frame_id, None, AccessibilityAction::SetSelection(0..11));
        });
        focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(frame_id), frame_mount);
            assert_eq!(host.focused(), Some(frame_id));
            let input = &host.text_surface(frame_id).unwrap().input;
            assert_eq!(input.text, current_frame);
            assert!(input.selected_text().is_empty());
            let len = input.len();
            host.accessibility_action(frame_id, None, AccessibilityAction::SetSelection(0..len));
            dispatch_content_window_event_with_text(
                host,
                &tekes_ui_platform_macos::Event::TextCommand(
                    tekes_ui_platform_macos::TextEditCommand::Copy,
                ),
                &clipboard,
                &mut text,
            )
            .unwrap();
            assert_eq!(
                clipboard.clipboard_text().as_deref(),
                Some(current_frame.as_str())
            );
        });
    }
    #[test]
    fn apphost_workbench_readers_preserve_selection_and_specimen_focus() {
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let unpainted = state.borrow().selected_snapshot().unwrap();
        assert!(unpainted.geometry.is_none());
        assert!(unpainted.last_paint.is_none());
        let body = format!(
            "{:?}",
            inspector_body(state.borrow().selection(), Some(unpainted))
        );
        assert!(body.contains("Unavailable (not rendered or invalidated)"));
        assert!(body.contains("Last paint (historical"));
        assert!(body.contains("Not recorded yet"));
        let mut text = super::super::text::TextEngine::new();
        focus_container(state.clone()).with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.focus(Some(ComponentId(100)), false));
            render_text_host(host, &mut text);
            assert_eq!(host.focused(), Some(ComponentId(100)));
        });
        let mut documents = panel("test-documents", "Documents", state.clone(), false).unwrap();
        let mut inspector = panel_source(state.clone(), true).unwrap();
        let mut version = 0;
        documents.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.text_surface(DOCUMENT_READER).unwrap().input.read_only);
            host.accessibility_action(
                DOCUMENT_READER,
                None,
                tekes_ui_controls::composition::AccessibilityAction::SetSelection(0..8),
            );
            version = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap()
                .version()
                .revision();
        });
        render_panel(&mut inspector, &mut text);
        for _ in 0..3 {
            inspector.with_host(&mut |_| {});
            documents.with_host(&mut |host| {
                assert_eq!(
                    host.text_surface(DOCUMENT_READER)
                        .unwrap()
                        .input
                        .selection(),
                    0..8
                );
                assert_eq!(
                    host.get(DOCUMENT_READER)
                        .unwrap()
                        .component::<RichContent>()
                        .unwrap()
                        .version()
                        .revision(),
                    version
                );
            });
        }
        let snapshot = state.borrow().selected_snapshot().unwrap();
        assert!(snapshot.focused);
        assert!(snapshot.geometry.is_some());
        assert!(snapshot.last_paint.is_some());
        let body = format!(
            "{:?}",
            inspector_body(state.borrow().selection(), Some(snapshot))
        );
        assert!(body.contains("Measured bounds, not declared sizing"));
        assert!(body.contains("Last paint (historical"));
    }
    #[test]
    fn apphost_workbench_rich_manual_preserves_code_selection_until_document_switch() {
        use tekes_ui_controls::composition::{AccessibilityAction, RichBlockKind};
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let mut panel = panel("documents", "Documents", state.clone(), false).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let mut mount = None;
        let mut copied = String::new();
        panel.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let rich = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap();
            assert!(
                rich.document()
                    .blocks()
                    .iter()
                    .any(|b| matches!(b.kind(), RichBlockKind::Heading { .. }))
            );
            let code = rich
                .document()
                .blocks()
                .iter()
                .find(|b| b.kind() == RichBlockKind::Code)
                .unwrap()
                .display();
            assert!(!rich.block_geometry().is_empty());
            host.accessibility_action(
                DOCUMENT_READER,
                None,
                AccessibilityAction::SetSelection(code),
            );
            copied = host
                .text_surface(DOCUMENT_READER)
                .unwrap()
                .input
                .selected_text();
            assert!(copied.contains("TextFieldContent::new"));
            assert!(!copied.contains("```"));
            mount = host.semantic_mount(DOCUMENT_READER);
        });
        state.borrow_mut().select(Some("owner"));
        panel.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
            assert_eq!(
                host.text_surface(DOCUMENT_READER)
                    .unwrap()
                    .input
                    .selected_text(),
                copied
            );
            let source = format!(
                "{}\n\nUpdated guidance",
                document_source("tui.text-field").unwrap()
            );
            publish_document(host, "tui.text-field", &source, &Cell::new(None));
            assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
            assert_eq!(
                host.text_surface(DOCUMENT_READER)
                    .unwrap()
                    .input
                    .selected_text(),
                copied
            );
            assert_eq!(
                host.get(DOCUMENT_READER)
                    .unwrap()
                    .component::<RichContent>()
                    .unwrap()
                    .version()
                    .revision(),
                2
            );
        });
        state.borrow_mut().select(Some("published"));
        panel.with_host(&mut |host| {
            assert_ne!(host.semantic_mount(DOCUMENT_READER), mount);
            let rich = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap();
            assert_eq!(rich.version().identity(), "tui.checkbox");
            assert_eq!(rich.version().revision(), 1);
            assert!(
                host.text_surface(DOCUMENT_READER)
                    .unwrap()
                    .input
                    .selected_text()
                    .is_empty()
            );
        });
    }

    #[test]
    fn apphost_workbench_selected_chooser_activation_returns_linked_reader() {
        use tekes_ui_controls::composition::AccessibilityAction;
        // Both panel choosers publish the same explicit return request.
        for from_inspector in [false, true] {
            let state = state(FORM_ROUTE);
            state.borrow_mut().select(Some("owner"));
            let mut documents = panel_source(state.clone(), false).unwrap();
            let mut inspector = panel_source(state.clone(), true).unwrap();
            let mut text = super::super::text::TextEngine::new();
            let mut linked_mount = None;
            documents.with_host(&mut |host| {
                publish_document(
                    host,
                    "tui.number-field",
                    document_source("tui.number-field").unwrap(),
                    &Cell::new(None),
                );
                render_text_host(host, &mut text);
                linked_mount = host.semantic_mount(DOCUMENT_READER);
            });
            // A same-value selection or ordinary callback/read is not activation.
            state.borrow_mut().select(Some("owner"));
            consume_chooser_actions(&state, vec![]);
            documents.with_host(&mut |host| {
                assert_eq!(host.semantic_mount(DOCUMENT_READER), linked_mount);
            });
            render_panel(&mut inspector, &mut text);
            let chooser = if from_inspector {
                &mut inspector
            } else {
                &mut documents
            };
            chooser.with_host(&mut |host| {
                render_text_host(host, &mut text);
                let list = host
                    .get(CHOOSER)
                    .unwrap()
                    .component::<KeyedSelectableListContent>()
                    .unwrap();
                let key = list.selected_key().unwrap();
                let part = list.semantic_part_for_key(key).unwrap();
                assert!(
                    host.accessibility_action(CHOOSER, Some(part), AccessibilityAction::Press)
                        .consumed
                );
                let records = host.drain_action_records();
                assert!(records.iter().any(|record| matches!(
                    &record.payload,
                    Some(ContentActionPayload::ListIntent(intent))
                        if intent.kind == ListIntentKind::Activated
                )));
                consume_chooser_actions(&state, records);
            });
            // Reading Inspector must not consume the Documents request.
            inspector.with_host(&mut |_| {});
            let mut returned_mount = None;
            documents.with_host(&mut |host| {
                returned_mount = host.semantic_mount(DOCUMENT_READER);
                assert_ne!(returned_mount, linked_mount);
                assert_eq!(
                    host.get(DOCUMENT_READER)
                        .unwrap()
                        .component::<RichContent>()
                        .unwrap()
                        .version()
                        .identity(),
                    "tui.text-field"
                );
            });
            documents.with_host(&mut |host| {
                assert_eq!(host.semantic_mount(DOCUMENT_READER), returned_mount);
            });
            assert_eq!(
                state.borrow().selection().specimen.as_deref(),
                Some("owner")
            );
        }
    }

    #[test]
    fn apphost_workbench_local_rich_link_uses_supplemental_identity_without_page_navigation() {
        use tekes_ui_controls::composition::{AccessibilityAction, Component};
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let selection = state.borrow().selection().clone();
        let mut panel = panel("documents", "Documents", state.clone(), false).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let mut mount = None;
        let document_focus = Cell::new(None);
        let source = RichDocument::from_markdown(document_source("tui.number-field").unwrap());
        let target = source
            .links()
            .iter()
            .find(|link| link.target() == "ticking-number.md")
            .unwrap()
            .target();
        panel.with_host(&mut |host| {
            // Use the real bundled target in a short document so the link is visible.
            publish_document(
                host,
                "tui.number-field",
                &format!("[Ticking Number]({target})"),
                &document_focus,
            );
            render_text_host(host, &mut text);
            mount = host.semantic_mount(DOCUMENT_READER);
            let part = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap()
                .semantic_parts()[0]
                .index;
            assert!(
                host.accessibility_action(DOCUMENT_READER, Some(part), AccessibilityAction::Press)
                    .consumed
            );
            assert!(host.drain_action_records().is_empty());
        });
        // Reads/paint must not consume the independent link queue or navigate.
        panel.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
            assert_eq!(
                host.get(DOCUMENT_READER)
                    .unwrap()
                    .component::<RichContent>()
                    .unwrap()
                    .version()
                    .identity(),
                "tui.number-field"
            );
            consume_document_links(host, &document_focus);
            assert_ne!(host.semantic_mount(DOCUMENT_READER), mount);
            let rich = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap();
            assert_eq!(rich.version().identity(), "supplemental.ticking-number");
            assert!(rich.document().source().contains("TickingNumber::new"));
            mount = host.semantic_mount(DOCUMENT_READER);
            consume_document_links(host, &document_focus);
            assert_eq!(
                host.semantic_mount(DOCUMENT_READER),
                mount,
                "link delivered only once"
            );
        });
        state.borrow_mut().select(Some("owner"));
        panel.with_host(&mut |host| {
            assert_ne!(host.semantic_mount(DOCUMENT_READER), mount);
            let rich = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap();
            assert_eq!(rich.version().identity(), "tui.text-field");
            assert_eq!(
                rich.document().source(),
                document_source("tui.text-field").unwrap()
            );
        });
        assert_eq!(state.borrow().selection().page, selection.page);
        assert!(state.borrow().pending.is_empty());
        assert_eq!(
            linked_document("supplemental.ticking-number", "number-field.md")
                .unwrap()
                .0,
            "tui.number-field"
        );
        assert!(document_source("supplemental.ticking-number").is_none());
    }

    #[test]
    fn apphost_workbench_link_reader_focus_waits_for_host_layout_then_accepts_keyboard() {
        use tekes_ui_controls::composition::{
            Component, InputModifiers, PointerButton, PointerEvent, PointerPhase,
        };
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let selection = state.borrow().selection().clone();
        let document_focus = Rc::new(Cell::new(None));
        let mut host = ContentHost::new(stack(
            20,
            vec![
                Content::new(DOCUMENT_NOTICE, TextContent::new(DOCUMENT_HELP)),
                Content::new(ComponentId(24), TextFieldContent::new("Other", "")),
                document_scroll("tui.text-field", "[NumberField](number-field.md)"),
            ],
        ))
        .unwrap();
        host.get_mut(ComponentId(20))
            .unwrap()
            .component_mut::<StackContent>()
            .unwrap()
            .flexible_child = Some(DOCUMENT_SCROLL);
        let mut panel = PanelContent {
            host,
            state,
            selection: Some(selection),
            inspector: false,
            inspector_groups: Vec::new(),
            inspector_target: None,
            document_focus: document_focus.clone(),
        };
        let mut text = super::super::text::TextEngine::new();
        panel.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.focus(Some(DOCUMENT_READER), false));
            let point = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap()
                .semantic_parts()[0]
                .frame
                .center();
            assert!(
                host.event(&ContentEvent::Pointer(PointerEvent {
                    position: point,
                    phase: PointerPhase::Down,
                    button: Some(PointerButton::Primary),
                    modifiers: InputModifiers {
                        command: true,
                        ..Default::default()
                    },
                    click_count: 1,
                }))
                .consumed
            );
            consume_document_links(host, &document_focus);
            assert!(document_focus.get().is_some());
            assert!(
                !host.focus(Some(DOCUMENT_READER), false),
                "unarranged replacement must not claim focus"
            );
            assert_eq!(host.focused(), None);
        });
        assert!(
            document_focus.get().is_some(),
            "plain host access cannot complete layout-dependent focus"
        );
        panel.with_host(&mut |host| render_text_host(host, &mut text));
        assert_eq!(document_focus.get(), None);
        panel.with_host(&mut |host| {
            assert_eq!(host.focused(), Some(DOCUMENT_READER));
            assert_eq!(
                host.get(DOCUMENT_READER)
                    .unwrap()
                    .component::<RichContent>()
                    .unwrap()
                    .version()
                    .identity(),
                "tui.number-field"
            );
            host.event(&ContentEvent::TextCommand(
                tekes_ui_controls::TextEditCommand::MoveToBeginningOfDocument,
            ));
            host.event(&ContentEvent::TextCommand(
                tekes_ui_controls::TextEditCommand::MoveToEndOfDocumentAndModifySelection,
            ));
            let input = &host.text_surface(DOCUMENT_READER).unwrap().input;
            assert_eq!(input.selected_text(), input.text);
            assert!(!input.selected_text().is_empty());
            // A user choosing another focus owner before the transfer completes
            // must not have that choice overwritten by the pending reader.
            publish_document(
                host,
                "tui.text-field",
                document_source("tui.text-field").unwrap(),
                &document_focus,
            );
            render_text_host(host, &mut text);
            assert!(host.focus(Some(ComponentId(24)), false));
        });
        assert_eq!(document_focus.get(), None);
        panel.with_host(&mut |host| assert_eq!(host.focused(), Some(ComponentId(24))));
    }

    #[test]
    fn apphost_workbench_unbundled_link_reports_without_replacing_reader() {
        use tekes_ui_controls::composition::{AccessibilityAction, Component};
        let mut panel = panel("documents", "Documents", state(FORM_ROUTE), false).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let document_focus = Cell::new(None);
        for target in ["https://example.com/manual", "missing.md"] {
            panel.with_host(&mut |host| {
                publish_document(
                    host,
                    "tui.number-field",
                    &format!("[Manual]({target})"),
                    &document_focus,
                );
                render_text_host(host, &mut text);
                let mount = host.semantic_mount(DOCUMENT_READER);
                let part = host
                    .get(DOCUMENT_READER)
                    .unwrap()
                    .component::<RichContent>()
                    .unwrap()
                    .semantic_parts()[0]
                    .index;
                host.accessibility_action(DOCUMENT_READER, Some(part), AccessibilityAction::Press);
                consume_document_links(host, &document_focus);
                assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
                let notice = &host
                    .get(DOCUMENT_NOTICE)
                    .unwrap()
                    .component::<TextContent>()
                    .unwrap()
                    .text;
                assert!(notice.contains("Link not opened"));
                assert!(notice.contains(target));
            });
        }
    }
    #[test]
    fn apphost_workbench_reenables_inspection_after_same_id_remount() {
        use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
        use tekes_ui_scene::{Rect, Scene, Size};
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, role: TextRole, width: f32) -> Size {
                Size::new(60_f32.min(width), role.style().line_height)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: tekes_ui_tokens::Rgba,
            ) {
            }
        }
        let state = state(FORM_ROUTE);
        // Keep only the selected original control so this regression needs no
        // native text resources. Replacements use the public host transaction.
        state.borrow_mut().pages.insert(
            FORM_ROUTE.into(),
            Rc::new(RefCell::new(
                ContentHost::new(stack(
                    200,
                    vec![Content::new(
                        ComponentId(101),
                        CheckboxContent::new("Published", CheckState::Unchecked),
                    )],
                ))
                .unwrap()
                .into(),
            )),
        );
        state.borrow_mut().select(Some("published"));
        let render = |state: &SharedWorkbench| {
            state.borrow_mut().with_selected_host(&mut |host, _| {
                let frame = Rect::new(0., 0., 240., 100.);
                host.render(
                    &mut Scene::new(),
                    &mut Text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
            });
        };
        state.borrow_mut().sync_inspection();
        render(&state);
        let original = state.borrow().selected_snapshot().unwrap();
        assert!(original.last_paint.is_some());
        let selection = state.borrow().selection().clone();
        state
            .borrow_mut()
            .with_selected_host(&mut |host, specimen| {
                host.remove(specimen.component_id).unwrap();
                host.insert(
                    ComponentId(200),
                    Content::new(
                        specimen.component_id,
                        CheckboxContent::new("Replacement", CheckState::Checked),
                    ),
                )
                .unwrap();
            });
        render(&state);
        let replacement = state.borrow().selected_snapshot().unwrap();
        assert_ne!(replacement.mount, original.mount);
        assert!(replacement.last_paint.is_none());
        state.borrow_mut().sync_inspection();
        render(&state);
        assert!(
            state
                .borrow()
                .selected_snapshot()
                .unwrap()
                .last_paint
                .is_some()
        );
        assert_eq!(*state.borrow().selection(), selection);
        state
            .borrow_mut()
            .with_selected_host(&mut |host, specimen| {
                host.remove(specimen.component_id).unwrap();
            });
        state.borrow_mut().sync_inspection();
        assert!(state.borrow().inspected.is_none());
        assert!(state.borrow().selected_snapshot().is_none());
    }
    #[test]
    fn apphost_display_progress_and_nested_panel_actions_keep_original_owners() {
        use tekes_ui_controls::composition::{AccessibilityAction, Button};
        for route in ["component-progress", "component-surface"] {
            let state = state(route);
            let mut focus = focus_container(state);
            let mut text = super::super::text::TextEngine::new();
            let count = if route == "component-progress" { 4 } else { 1 };
            for _ in 0..count {
                focus.with_host(&mut |host| {
                    render_text_host(host, &mut text);
                    assert!(
                        host.accessibility_action(
                            ComponentId(92120),
                            None,
                            AccessibilityAction::Press
                        )
                        .consumed
                    );
                });
            }
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                assert!(
                    !host
                        .accessibility_action(ComponentId(96020), None, AccessibilityAction::Press)
                        .consumed,
                    "passive owner is not the nested action"
                );
                let log = &host
                    .get(ComponentId(92003))
                    .unwrap()
                    .component::<TextContent>()
                    .unwrap()
                    .text;
                if route == "component-progress" {
                    assert_eq!(log.matches("progress:").count(), 4);
                    assert_eq!(
                        host.get(ComponentId(96021))
                            .unwrap()
                            .component::<TextContent>()
                            .unwrap()
                            .text,
                        "100%"
                    );
                    assert!(
                        host.get(ComponentId(92120))
                            .unwrap()
                            .component::<Button>()
                            .unwrap()
                            .button
                            .state
                            .disabled
                    );
                    assert!(
                        !host
                            .accessibility_action(
                                ComponentId(92120),
                                None,
                                AccessibilityAction::Press
                            )
                            .consumed
                    );
                } else {
                    assert_eq!(log.matches("activate: primary action").count(), 1);
                }
            });
        }
    }

    #[test]
    fn apphost_section_bar_keeps_one_row_and_reveals_last_command_at_narrow_width() {
        use tekes_ui_controls::composition::AccessibilityAction;
        for width in [740., 280.] {
            let state = state("component-button");
            let mut focus = focus_container(state);
            let mut text = super::super::text::TextEngine::new();
            text.set_type_scale(1.5);
            let frame = tekes_ui_scene::Rect::new(0., 0., width, 900.);
            focus.with_host(&mut |host| {
                host.render(
                    &mut tekes_ui_scene::Scene::new(),
                    &mut text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
                let first = host.get(ComponentId(92101)).unwrap().frame();
                let last = host.get(ComponentId(92100)).unwrap().frame();
                assert!(
                    (first.min_y() - last.min_y()).abs() < 0.1,
                    "section commands formed a vertical column"
                );
                assert!(
                    host.get(ComponentId(92006)).unwrap().frame().size.height
                        < first.size.height * 2.
                );
                assert_eq!(
                    host.get(ComponentId(92004))
                        .unwrap()
                        .component::<TextContent>()
                        .unwrap()
                        .role,
                    TextRole::UiHeading
                );
                assert!(host.focus(Some(ComponentId(92100)), true));
                host.render(
                    &mut tekes_ui_scene::Scene::new(),
                    &mut text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
                let last = host.get(ComponentId(92100)).unwrap().frame();
                let viewport = host.get(ComponentId(92012)).unwrap().frame();
                assert!(
                    last.min_x() >= viewport.min_x() - 0.1
                        && last.max_x() <= viewport.max_x() + 0.1,
                    "Reset remains outside the horizontal viewport"
                );
                assert!(
                    host.accessibility_action(ComponentId(92100), None, AccessibilityAction::Press)
                        .consumed
                );
            });
        }
    }

    #[test]
    fn apphost_reading_pages_measure_original_documents_inside_narrow_focus() {
        use tekes_ui_controls::composition::AccessibilityAction;
        for route in [
            "component-markdown",
            "component-code-block",
            "component-diff-view",
        ] {
            let state = state(route);
            let mut focus = focus_container(state);
            let mut text = super::super::text::TextEngine::new();
            text.set_type_scale(1.5);
            let frame = tekes_ui_scene::Rect::new(0., 0., 320., 900.);
            focus.with_host(&mut |host| {
                host.render(
                    &mut tekes_ui_scene::Scene::new(),
                    &mut text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
                let document = host.get(ComponentId(96020)).unwrap().frame();
                let footer = host.get(ComponentId(96021)).unwrap().frame();
                assert!(document.size.height > 0. && document.size.width <= frame.size.width);
                assert!(
                    footer.min_y() >= document.max_y(),
                    "document text collides with its original footer"
                );
                assert!(
                    !host
                        .accessibility_action(ComponentId(96020), None, AccessibilityAction::Press)
                        .consumed
                );
            });
        }
    }

    #[test]
    fn apphost_selection_pages_preserve_real_changes_across_passive_sections() {
        use tekes_ui_controls::composition::{
            AccessibilityAction, Component, RadioGroupContent, SliderContent, SwitchContent,
        };
        for route in [
            "component-switch",
            "component-slider",
            "component-radio-group",
        ] {
            let state = state(route);
            let mut focus = focus_container(state);
            let mut text = super::super::text::TextEngine::new();
            let specimen = workbench::apphost_basic::SPECIMEN;
            let action = if route == "component-slider" {
                AccessibilityAction::Increment
            } else {
                AccessibilityAction::Press
            };
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                let part = host
                    .get(specimen)
                    .unwrap()
                    .component::<RadioGroupContent>()
                    .map(|radio| radio.semantic_parts()[1].index);
                assert!(
                    host.accessibility_action(specimen, part, action.clone())
                        .consumed
                );
            });
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                assert!(
                    host.accessibility_action(ComponentId(92110), None, AccessibilityAction::Press)
                        .consumed
                );
            });
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                let part = host
                    .get(specimen)
                    .unwrap()
                    .component::<RadioGroupContent>()
                    .and_then(|radio| radio.semantic_parts().get(1).map(|part| part.index));
                assert!(
                    !host
                        .accessibility_action(specimen, part, action.clone())
                        .consumed
                );
                assert!(
                    host.accessibility_action(ComponentId(92103), None, AccessibilityAction::Press)
                        .consumed
                );
            });
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                assert!(host.get(specimen).is_none());
                assert!(
                    !host
                        .accessibility_action(ComponentId(92801), None, AccessibilityAction::Press)
                        .consumed
                );
                assert!(
                    host.accessibility_action(ComponentId(92102), None, AccessibilityAction::Press)
                        .consumed
                );
            });
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                let content = host.get(specimen).unwrap();
                match route {
                    "component-switch" => {
                        let c = content.component::<SwitchContent>().unwrap();
                        assert!(c.model.checked && c.model.disabled);
                    }
                    "component-slider" => {
                        let c = content.component::<SliderContent>().unwrap();
                        assert_eq!(c.model.value(), 55.);
                        assert!(c.model.disabled);
                    }
                    _ => assert_eq!(
                        content
                            .component::<RadioGroupContent>()
                            .unwrap()
                            .model()
                            .selected,
                        Some(1)
                    ),
                }
                assert!(
                    host.accessibility_action(ComponentId(92100), None, AccessibilityAction::Press)
                        .consumed
                );
            });
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                let content = host.get(specimen).unwrap();
                match route {
                    "component-switch" => {
                        let c = content.component::<SwitchContent>().unwrap();
                        assert!(!c.model.checked && !c.model.disabled);
                    }
                    "component-slider" => {
                        let c = content.component::<SliderContent>().unwrap();
                        assert_eq!(c.model.value(), 50.);
                        assert!(!c.model.disabled);
                    }
                    _ => assert_eq!(
                        content
                            .component::<RadioGroupContent>()
                            .unwrap()
                            .model()
                            .selected,
                        Some(0)
                    ),
                }
            });
        }
    }

    #[test]
    fn apphost_navigation_deactivates_departing_cached_host_in_its_existing_borrow() {
        let state = state(FORM_ROUTE);
        let old = state.borrow().pages[FORM_ROUTE].clone();
        assert!(old.borrow().is_window_active());
        publish(&state, SEARCH_ROUTE);
        assert!(!old.borrow().is_window_active());
        assert!(
            state.borrow().pages[SEARCH_ROUTE]
                .borrow()
                .is_window_active()
        );
        state.borrow_mut().restore(COMMAND).unwrap();
        let mut text = super::super::text::TextEngine::new();
        focus_container(state).with_host(&mut |host| render_text_host(host, &mut text));
        assert!(old.borrow().is_window_active());
    }

    #[test]
    fn apphost_scene3d_specimen_uses_its_page_guidance_without_a_fictitious_manual() {
        let selection = Selection::new("component-scene3d", None);
        assert_eq!(selection.specimen.as_deref(), Some("original-scene3d"));
        assert!(selection.document_id.is_none());
        assert!(document_body(&selection).contains("glTF"));
        assert!(!document_body(&selection).contains("has not migrated"));
        assert_eq!(document_key(&selection), "page:component-scene3d");
        assert!(
            page_host(
                "component-scene3d",
                &Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)))
            )
            .unwrap()
            .get(workbench::apphost_scene3d::SPECIMEN)
            .is_some()
        );
    }

    #[test]
    fn apphost_button_loading_rejects_action_and_sections_preserve_fixture_state() {
        use tekes_ui_controls::composition::{AccessibilityAction, Button};
        let state = state("component-button");
        let mut focus = focus_container(state);
        let mut text = super::super::text::TextEngine::new();
        let specimen = workbench::apphost_basic::SPECIMEN;
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(
                host.accessibility_action(specimen, None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            let log = &host
                .get(ComponentId(92003))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text;
            assert_eq!(log.matches("activate: primary action").count(), 1);
            render_text_host(host, &mut text);
            assert!(
                host.accessibility_action(ComponentId(92111), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(
                host.get(specimen)
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .button
                    .state
                    .loading
            );
            assert!(
                !host
                    .accessibility_action(specimen, None, AccessibilityAction::Press)
                    .consumed
            );
            assert!(
                host.accessibility_action(ComponentId(92106), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.get(specimen).is_none());
            // The original sized examples are passive; no accidental action
            // owner is introduced when wrapping the old Button renderer.
            assert!(
                !host
                    .accessibility_action(ComponentId(92400), None, AccessibilityAction::Press)
                    .consumed
            );
            assert!(
                host.accessibility_action(ComponentId(92105), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(
                host.get(specimen)
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .button
                    .state
                    .loading
            );
            assert!(
                host.accessibility_action(ComponentId(92111), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert_eq!(
                host.get(specimen)
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .label,
                "A long action label 中日文"
            );
            assert!(
                host.accessibility_action(specimen, None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            let log = &host
                .get(ComponentId(92003))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text;
            assert_eq!(log.matches("activate: primary action").count(), 2);
        });
    }

    #[test]
    fn apphost_index_activation_enters_the_original_registry_route_once() {
        use tekes_ui_controls::composition::AccessibilityAction;
        let state = state("component-index");
        let mut focus = focus_container(state.clone());
        let mut text = super::super::text::TextEngine::new();
        let index = workbench::entries()
            .iter()
            .position(|e| e.slug() == "component-stack")
            .unwrap();
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(
                host.accessibility_action(
                    ComponentId(94000 + index as u64),
                    None,
                    AccessibilityAction::Press
                )
                .consumed
            );
        });
        let request = FocusContent(state.clone())
            .take_navigation_request()
            .unwrap();
        assert!(
            FocusContent(state.clone())
                .take_navigation_request()
                .is_none()
        );
        let visit = request.params["visit"].as_u64().unwrap();
        state.borrow_mut().restore(COMMAND + visit).unwrap();
        assert_eq!(state.borrow().selection().page, "component-stack");
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.get(ComponentId(92601)).is_some());
            assert!(
                host.accessibility_action(ComponentId(92106), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.get(ComponentId(92600)).is_some());
            assert!(host.get(ComponentId(92601)).is_some());
            assert!(host.get(ComponentId(92602)).is_some());
            assert!(
                !host
                    .accessibility_action(ComponentId(92601), None, AccessibilityAction::Press)
                    .consumed
            );
        });
    }

    #[test]
    fn apphost_checkbox_sections_preserve_live_model_and_disable_without_remount() {
        use tekes_ui_controls::composition::AccessibilityAction;
        let state = state("component-checkbox");
        let mut focus = focus_container(state.clone());
        let mut text = super::super::text::TextEngine::new();
        let specimen = workbench::apphost_basic::SPECIMEN;
        {
            let mut press = |target| {
                focus.with_host(&mut |host| {
                    render_text_host(host, &mut text);
                    assert!(
                        host.accessibility_action(target, None, AccessibilityAction::Press)
                            .consumed
                    );
                });
            };
            // Mixed is a fixture command; activating the real checkbox must toggle
            // exactly once, not replay through Gallery's old activation reducer.
            press(ComponentId(92113));
            press(specimen);
        }
        focus.with_host(&mut |host| {
            assert_eq!(
                host.get(specimen)
                    .unwrap()
                    .component::<CheckboxContent>()
                    .unwrap()
                    .model()
                    .checked,
                CheckState::Checked
            );
        });
        let mut mount = None;
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            mount = host.semantic_mount(specimen);
            assert!(
                host.accessibility_action(ComponentId(92110), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert_eq!(host.semantic_mount(specimen), mount);
            assert!(
                !host
                    .accessibility_action(specimen, None, AccessibilityAction::Press)
                    .consumed
            );
            assert_eq!(
                host.get(specimen)
                    .unwrap()
                    .component::<CheckboxContent>()
                    .unwrap()
                    .model()
                    .checked,
                CheckState::Checked
            );
            assert!(
                host.accessibility_action(ComponentId(92103), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        // The state matrix is passive and retires the interactive mount.
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.get(specimen).is_none());
            assert!(
                !host
                    .accessibility_action(specimen, None, AccessibilityAction::Press)
                    .consumed
            );
            assert!(
                host.accessibility_action(ComponentId(92105), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let control = host
                .get(specimen)
                .unwrap()
                .component::<CheckboxContent>()
                .unwrap();
            assert_eq!(control.model().checked, CheckState::Checked);
            assert!(control.model().button.state.disabled);
            assert_ne!(host.semantic_mount(specimen), mount);
        });
        // Leave the route and return: the live section/model owner is retained.
        publish(&state, SEARCH_ROUTE);
        state.borrow_mut().restore(COMMAND).unwrap();
        focus.with_host(&mut |host| {
            assert_eq!(
                host.get(ComponentId(92007))
                    .unwrap()
                    .component::<TextContent>()
                    .unwrap()
                    .text,
                "Edge cases"
            );
            assert_eq!(
                host.get(specimen)
                    .unwrap()
                    .component::<CheckboxContent>()
                    .unwrap()
                    .model()
                    .checked,
                CheckState::Checked
            );
            render_text_host(host, &mut text);
            assert!(
                host.accessibility_action(ComponentId(92100), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            let control = host
                .get(specimen)
                .unwrap()
                .component::<CheckboxContent>()
                .unwrap();
            assert_eq!(control.model().checked, CheckState::Unchecked);
            assert!(!control.model().button.state.disabled);
            assert_eq!(
                host.get(ComponentId(92007))
                    .unwrap()
                    .component::<TextContent>()
                    .unwrap()
                    .text,
                "Edge cases"
            );
        });
    }

    #[test]
    fn apphost_original_pages_preserve_edits_and_specimen_selection_on_history() {
        let state = state(AREA_ROUTE);
        let mut text = super::super::text::TextEngine::new();
        focus_container(state.clone()).with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.focus(Some(ComponentId(27502)), false));
            render_text_host(host, &mut text);
            host.event_with_text(
                &ContentEvent::Commit {
                    text: " retained area edit".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
        });
        let original_mount = state.borrow().selected_snapshot().unwrap().mount;
        for route in [COMBO_ROUTE, SELECT_ROUTE, FORM_ROUTE] {
            publish(&state, route);
        }
        state.borrow_mut().select(Some("original-field"));
        assert_eq!(
            state.borrow().selected_snapshot().unwrap().id,
            ComponentId(27501)
        );
        state.borrow_mut().restore(COMMAND).unwrap();
        assert_eq!(
            state.borrow().selection().document_id.as_deref(),
            Some("tui.text-area")
        );
        focus_container(state.clone()).with_host(&mut |host| {
            assert!(
                host.text_surface(ComponentId(27502))
                    .unwrap()
                    .input
                    .text
                    .contains("retained area edit")
            );
        });
        assert_eq!(
            state.borrow().selected_snapshot().unwrap().mount,
            original_mount
        );
        state.borrow_mut().restore(COMMAND + 3).unwrap();
        assert_eq!(
            state.borrow().selection().specimen.as_deref(),
            Some("original-field")
        );
    }
}
