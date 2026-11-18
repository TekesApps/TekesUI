use super::workbench;
use std::collections::HashMap;
use tekes_ui_controls::composition::ComponentId;

pub(super) const SEARCH_ROUTE: &str = "component-search-field";
pub(super) const FORM_ROUTE: &str = "component-text-field";
pub(super) const AREA_ROUTE: &str = "component-text-area";
pub(super) const COMBO_ROUTE: &str = "component-combo-box";
pub(super) const SELECT_ROUTE: &str = "component-select";

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
pub(super) fn directory_page(page: &str) -> &str {
    if workbench::apphost_quality::category_for_slug(page).is_some() {
        "lab-consistency"
    } else {
        page
    }
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
    pub(super) fn new(page: &str, specimen: Option<&str>) -> Self {
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
