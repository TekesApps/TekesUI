use crate::{Result, SettingsError};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::Index,
};

/// Map of settings override keys to typed [`Value`]s.
pub type Table = BTreeMap<String, Value>;

/// Owned, format-agnostic configuration value used in settings schemas and overrides.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Value {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Table(Table),
}

impl Value {
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }
    pub fn is_str(&self) -> bool {
        matches!(self, Self::String(_))
    }
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }
    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table(_))
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v.as_str()),
            _ => None,
        }
    }
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Self::Array(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Self::Table(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_table_mut(&mut self) -> Option<&mut Table> {
        match self {
            Self::Table(v) => Some(v),
            _ => None,
        }
    }
}

impl Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Self::Output {
        self.as_table()
            .and_then(|table| table.get(key))
            .expect("settings key lookup")
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Boolean(v)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::String(v.to_owned())
    }
}
impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Self::Array(v)
    }
}
impl From<Table> for Value {
    fn from(v: Table) -> Self {
        Self::Table(v)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean(v) => fmt::Display::fmt(v, f),
            Self::Integer(v) => fmt::Display::fmt(v, f),
            Self::Float(v) => fmt::Display::fmt(v, f),
            Self::String(v) => f.write_str(v),
            Self::Array(_) | Self::Table(_) => match to_toml_value(self) {
                Ok(value) => f.write_str(&value.to_string()),
                Err(_) => f.write_str("<invalid>"),
            },
        }
    }
}

pub(crate) fn from_toml_value(value: toml::Value) -> Result<Value> {
    match value {
        toml::Value::Boolean(v) => Ok(Value::Boolean(v)),
        toml::Value::Integer(v) => Ok(Value::Integer(v)),
        toml::Value::Float(v) => Ok(Value::Float(v)),
        toml::Value::String(v) => Ok(Value::String(v)),
        toml::Value::Array(items) => items
            .into_iter()
            .map(from_toml_value)
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        toml::Value::Table(map) => from_toml_table(map).map(Value::Table),
        toml::Value::Datetime(dt) => Ok(Value::String(dt.to_string())),
    }
}

pub(crate) fn from_toml_table(map: toml::Table) -> Result<Table> {
    map.into_iter()
        .map(|(k, v)| Ok((k, from_toml_value(v)?)))
        .collect()
}

pub(crate) fn to_toml_value(value: &Value) -> Result<toml::Value> {
    match value {
        Value::Boolean(v) => Ok(toml::Value::Boolean(*v)),
        Value::Integer(v) => Ok(toml::Value::Integer(*v)),
        Value::Float(v) => Ok(toml::Value::Float(*v)),
        Value::String(v) => Ok(toml::Value::String(v.clone())),
        Value::Array(items) => items
            .iter()
            .map(to_toml_value)
            .collect::<Result<Vec<_>>>()
            .map(toml::Value::Array),
        Value::Table(map) => to_toml_table(map).map(toml::Value::Table),
    }
}

pub(crate) fn to_toml_table(map: &Table) -> Result<toml::Table> {
    map.iter()
        .map(|(k, v)| Ok((k.clone(), to_toml_value(v)?)))
        .collect()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub schema_version: u32,
    pub namespace: String,
    pub provider: String,
    #[serde(default)]
    pub pages: Vec<Page>,
    pub groups: Vec<Group>,
    #[serde(default)]
    pub settings: Vec<Field>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    #[serde(default)]
    pub page: String,
    pub label: String,
    pub parent: Option<String>,
    #[serde(default)]
    pub items: Vec<Item>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub label: String,
    pub fields: Vec<Field>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub id: String,
    pub path: Vec<String>,
    #[serde(default)]
    pub group: String,
    #[serde(skip)]
    pub item: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(default)]
    pub presentation: FieldPresentation,
    pub default: Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub options: Vec<OptionValue>,
}
/// Optional UI choice; values and persistence retain their original field type.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldPresentation {
    #[default]
    Auto,
    Radio,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Boolean,
    String,
    Integer,
    Decimal,
    Enum,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionValue {
    pub value: String,
    pub label: String,
}
impl Schema {
    pub fn validate_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        for field in &self.settings {
            if let Some(value) = get(&snapshot.overrides, &field.path) {
                field.validate(value)?;
            }
        }
        Ok(())
    }
    pub fn items(&self) -> impl Iterator<Item = (String, &Item)> {
        self.groups.iter().flat_map(|g| {
            g.items
                .iter()
                .map(move |i| (format!("{}/{}", g.id, i.id), i))
        })
    }
    pub fn for_item(&self, item_id: &str) -> Result<Self> {
        let (_, item) = self
            .items()
            .find(|(id, _)| id == item_id)
            .ok_or_else(|| SettingsError::InvalidSchema("unknown Settings item".into()))?;
        let mut selected = self.clone();
        selected.pages[0].label = item.label.clone();
        selected.settings.retain(|f| f.item == item_id);
        let mut keep_groups: BTreeSet<String> =
            selected.settings.iter().map(|f| f.group.clone()).collect();
        let mut cursor: Vec<String> = keep_groups.iter().cloned().collect();
        while let Some(group_id) = cursor.pop() {
            if let Some(group) = self.groups.iter().find(|g| g.id == group_id)
                && let Some(parent_id) = &group.parent
                && keep_groups.insert(parent_id.clone())
            {
                cursor.push(parent_id.clone());
            }
        }
        selected.groups.retain(|g| keep_groups.contains(&g.id));
        Ok(selected)
    }

    pub fn parse(source: &str) -> Result<Self> {
        let mut schema: Self = toml::from_str(source)
            .map_err(|err| SettingsError::InvalidSchema(format!("Settings TOML: {err}")))?;
        if schema.pages.is_empty() {
            schema.pages.push(Page {
                id: "settings".into(),
                label: "通用".into(),
            });
        }
        for group in &mut schema.groups {
            if group.page.is_empty() {
                group.page = schema.pages[0].id.clone();
            }
        }
        let mut item_ids = BTreeSet::new();
        for group in &mut schema.groups {
            for item in &mut group.items {
                if !(!item.id.is_empty()
                    && !item.label.trim().is_empty()
                    && item_ids.insert(format!("{}/{}", group.id, item.id)))
                {
                    return Err(SettingsError::InvalidSchema(format!(
                        "invalid/duplicate item {}",
                        item.id
                    )));
                }
                let mut field_ids = BTreeSet::new();
                for field in &mut item.fields {
                    if field.id.is_empty() || !field_ids.insert(field.id.clone()) {
                        return Err(SettingsError::InvalidSchema(format!(
                            "duplicate item field {}",
                            field.id
                        )));
                    }
                    field.item = format!("{}/{}", group.id, item.id);
                    field.id = format!("{}/{}", field.item, field.id);
                    field.group = group.id.clone();
                    schema.settings.push(field.clone());
                }
            }
        }
        if schema.schema_version != 1 {
            return Err(SettingsError::InvalidSchema(
                "unsupported Settings schema version".into(),
            ));
        }
        if schema.namespace.trim().is_empty() || schema.provider.trim().is_empty() {
            return Err(SettingsError::InvalidSchema(
                "namespace/provider required".into(),
            ));
        }
        // The first form has one page; reject rather than silently losing other pages.
        if schema.pages.len() != 1 {
            return Err(SettingsError::InvalidSchema(
                "prototype supports exactly one Settings page".into(),
            ));
        }
        if schema.pages[0].id.is_empty() || schema.pages[0].label.trim().is_empty() {
            return Err(SettingsError::InvalidSchema(
                "page ID/label required".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for group in &schema.groups {
            if !(!group.id.is_empty() && !group.label.trim().is_empty() && ids.insert(&group.id)) {
                return Err(SettingsError::InvalidSchema(format!(
                    "invalid/duplicate group: {}",
                    group.id
                )));
            }
            if group.page != schema.pages[0].id {
                return Err(SettingsError::InvalidSchema(format!(
                    "unknown page for group {}",
                    group.id
                )));
            }
        }
        for group in &schema.groups {
            let mut visiting = BTreeSet::new();
            let mut current = Some(group);
            while let Some(g) = current {
                if !visiting.insert(&g.id) {
                    return Err(SettingsError::InvalidSchema(format!(
                        "cyclic Settings group: {}",
                        g.id
                    )));
                }
                current = if let Some(parent) = &g.parent {
                    Some(
                        schema
                            .groups
                            .iter()
                            .find(|g| &g.id == parent)
                            .ok_or_else(|| {
                                SettingsError::InvalidSchema(format!("unknown parent {parent}"))
                            })?,
                    )
                } else {
                    None
                };
            }
        }
        let mut field_ids = BTreeSet::new();
        let mut paths: Vec<&Vec<String>> = Vec::new();
        for field in &schema.settings {
            if !(!field.id.is_empty()
                && !field.label.trim().is_empty()
                && field_ids.insert(&field.id))
            {
                return Err(SettingsError::InvalidSchema(format!(
                    "invalid/duplicate field: {}",
                    field.id
                )));
            }
            if !ids.contains(&field.group) {
                return Err(SettingsError::InvalidSchema(format!(
                    "unknown group: {}",
                    field.group
                )));
            }
            if field.path.is_empty()
                || !field.path.iter().all(|p| !p.is_empty())
                || field.path[0] == "_tekes"
            {
                return Err(SettingsError::InvalidSchema(format!(
                    "invalid/reserved path: {}",
                    field.id
                )));
            }
            if !paths
                .iter()
                .all(|p| !p.starts_with(&field.path) && !field.path.starts_with(p))
            {
                return Err(SettingsError::InvalidSchema(format!(
                    "overlapping Settings path: {}",
                    field.id
                )));
            }
            paths.push(&field.path);
            if !(field.min.is_none_or(f64::is_finite) && field.max.is_none_or(f64::is_finite)) {
                return Err(SettingsError::InvalidSchema("invalid range".into()));
            }
            if !field.min.zip(field.max).is_none_or(|(a, b)| a <= b) {
                return Err(SettingsError::InvalidSchema("reversed range".into()));
            }
            if field.step.is_some() || !field.suffix.is_empty() {
                if !matches!(field.kind, FieldType::Integer | FieldType::Decimal) {
                    return Err(SettingsError::InvalidSchema(
                        "numeric format on nonnumeric field".into(),
                    ));
                }
                if field.min.is_none() || field.max.is_none() {
                    return Err(SettingsError::InvalidSchema(
                        "numeric control requires explicit bounds".into(),
                    ));
                }
                if !field.step.is_none_or(|s| s.is_finite() && s > 0.) {
                    return Err(SettingsError::InvalidSchema("invalid numeric step".into()));
                }
                if !(field.kind != FieldType::Integer || field.step.is_none_or(|s| s.fract() == 0.))
                {
                    return Err(SettingsError::InvalidSchema(
                        "integer step must be integral".into(),
                    ));
                }
            }
            if !(field.presentation != FieldPresentation::Radio || field.kind == FieldType::Enum) {
                return Err(SettingsError::InvalidSchema(format!(
                    "radio presentation requires enum field: {}",
                    field.id
                )));
            }
            let mut options = BTreeSet::new();
            if !field.options.iter().all(|o| {
                !o.value.is_empty() && !o.label.trim().is_empty() && options.insert(&o.value)
            }) {
                return Err(SettingsError::InvalidSchema(format!(
                    "invalid/duplicate option: {}",
                    field.id
                )));
            }
            field.validate(&field.default).map_err(|err| {
                SettingsError::InvalidSchema(format!("default {}: {err}", field.id))
            })?;
        }
        Ok(schema)
    }
}
impl Field {
    pub fn validate(&self, value: &Value) -> Result<()> {
        let valid = match self.kind {
            FieldType::Boolean => value.is_bool(),
            FieldType::String => value.is_str(),
            FieldType::Integer => value.is_integer(),
            FieldType::Decimal => value.is_float() || value.is_integer(),
            FieldType::Enum => value
                .as_str()
                .is_some_and(|v| self.options.iter().any(|o| o.value == v)),
        };
        if !valid {
            return Err(SettingsError::InvalidFieldValue(format!(
                "{}: invalid {:?} value",
                self.label, self.kind
            )));
        }
        let number = value
            .as_float()
            .or_else(|| value.as_integer().map(|v| v as f64));
        if let Some(n) = number
            && !(n.is_finite()
                && self.min.is_none_or(|m| n >= m)
                && self.max.is_none_or(|m| n <= m))
        {
            return Err(SettingsError::InvalidFieldValue(format!(
                "{}: value outside allowed range",
                self.label
            )));
        }
        Ok(())
    }
    pub fn parse_input(&self, raw: &str) -> Result<Value> {
        let value = match self.kind {
            FieldType::String | FieldType::Enum => Value::String(raw.into()),
            FieldType::Boolean => Value::Boolean(
                raw.parse()
                    .map_err(|_| SettingsError::InvalidFieldValue("expected boolean".into()))?,
            ),
            FieldType::Integer => Value::Integer(raw.trim().parse().map_err(|_| {
                SettingsError::InvalidFieldValue(format!("{}: expected integer", self.label))
            })?),
            FieldType::Decimal => Value::Float(raw.trim().parse().map_err(|_| {
                SettingsError::InvalidFieldValue(format!("{}: expected number", self.label))
            })?),
        };
        self.validate(&value)?;
        Ok(value)
    }
    pub fn display(&self, value: &Value) -> String {
        match value {
            Value::String(v) => v.clone(),
            _ => value.to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub revision: u64,
    pub last_commit_id: String,
    #[serde(with = "snapshot_table")]
    pub overrides: Table,
}
// JSON alone cannot preserve TOML datetime or non-finite float variants. The
// typed snapshot envelope carries its unknown-key payload as lossless TOML.
mod snapshot_table {
    use super::{Table, from_toml_table, to_toml_table};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &Table, serializer: S) -> Result<S::Ok, S::Error> {
        let toml_table = to_toml_table(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&toml::to_string(&toml_table).map_err(serde::ser::Error::custom)?)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Table, D::Error> {
        let source = String::deserialize(deserializer)?;
        let toml_table: toml::Table = toml::from_str(&source).map_err(serde::de::Error::custom)?;
        from_toml_table(toml_table).map_err(serde::de::Error::custom)
    }
}
#[derive(Clone, Debug, PartialEq)]
enum Edit {
    Input(String),
    Unset,
}
#[derive(Clone, Debug)]
pub struct Draft {
    pub base: Snapshot,
    edits: BTreeMap<String, Edit>,
    pub generation: u64,
    edit_session: u64,
    versions: BTreeMap<String, u64>,
}
#[derive(Clone, Debug)]
pub struct Submission {
    pub commit_id: String,
    pub expected_revision: u64,
    edit_session: u64,
    pub overrides: Table,
    edits: BTreeMap<String, Edit>,
    versions: BTreeMap<String, u64>,
}
impl Submission {
    /// Restore a complete saved snapshot through the same single-writer Provider.
    /// History still requires its own durable confirmation after this write.
    pub fn replacement(before: &Snapshot, after: &Snapshot) -> Result<Self> {
        let expected = before
            .revision
            .checked_add(1)
            .ok_or_else(|| SettingsError::RevisionConflict("revision exhausted".into()))?;
        if after.revision != expected || after.last_commit_id.is_empty() {
            return Err(SettingsError::DraftState(
                "invalid replacement identity".into(),
            ));
        }
        Ok(Self {
            commit_id: after.last_commit_id.clone(),
            expected_revision: before.revision,
            overrides: after.overrides.clone(),
            edit_session: 0,
            edits: BTreeMap::new(),
            versions: BTreeMap::new(),
        })
    }
}
impl Draft {
    pub fn new(base: Snapshot) -> Self {
        Self {
            base,
            edits: BTreeMap::new(),
            generation: 0,
            edit_session: 0,
            versions: BTreeMap::new(),
        }
    }
    pub fn dirty(&self) -> bool {
        !self.edits.is_empty()
    }
    pub fn display(&self, field: &Field) -> String {
        match self.edits.get(&field.id) {
            Some(Edit::Input(v)) => v.clone(),
            Some(Edit::Unset) => field.display(&field.default),
            None => field.display(get(&self.base.overrides, &field.path).unwrap_or(&field.default)),
        }
    }
    pub fn edit(&mut self, field: &Field, raw: String) {
        if raw == self.display(field) {
            return;
        }
        let original =
            field.display(get(&self.base.overrides, &field.path).unwrap_or(&field.default));
        if raw == original {
            self.edits.remove(&field.id);
        } else {
            self.edits.insert(field.id.clone(), Edit::Input(raw));
        }
        self.generation += 1;
        self.versions.insert(field.id.clone(), self.generation);
    }
    pub fn reset(&mut self, schema: &Schema) {
        for f in &schema.settings {
            if get(&self.base.overrides, &f.path).is_some() {
                self.edits.insert(f.id.clone(), Edit::Unset);
            } else {
                self.edits.remove(&f.id);
            }
        }
        self.generation += 1;
        for f in &schema.settings {
            self.versions.insert(f.id.clone(), self.generation);
        }
    }
    pub fn cancel(&mut self) {
        self.edits.clear();
        self.generation += 1;
        self.edit_session += 1;
        self.versions.clear();
    }
    pub fn submit(&self, schema: &Schema, commit_id: String) -> Result<Submission> {
        if !self.dirty() {
            return Err(SettingsError::DraftState("no changes to apply".into()));
        }
        let mut overrides = self.base.overrides.clone();
        for f in &schema.settings {
            match self.edits.get(&f.id) {
                Some(Edit::Input(raw)) => set(&mut overrides, &f.path, Some(f.parse_input(raw)?))?,
                Some(Edit::Unset) => set(&mut overrides, &f.path, None)?,
                None => {}
            }
        }
        Ok(Submission {
            commit_id,
            expected_revision: self.base.revision,
            edit_session: self.edit_session,
            overrides,
            edits: self.edits.clone(),
            versions: self.versions.clone(),
        })
    }
    pub fn accept(
        &mut self,
        schema: &Schema,
        submission: &Submission,
        snapshot: Snapshot,
    ) -> Result<()> {
        if !(snapshot.last_commit_id == submission.commit_id
            && snapshot.revision == submission.expected_revision + 1
            && snapshot.overrides == submission.overrides)
        {
            return Err(SettingsError::RevisionConflict(
                "mismatched Settings receipt".into(),
            ));
        }
        // Only retire edits in this batch. Newer typing stays dirty.
        let same_session = self.edit_session == submission.edit_session;
        let newer: Vec<_> = schema
            .settings
            .iter()
            .filter(|f| {
                if same_session {
                    self.versions.get(&f.id) != submission.versions.get(&f.id)
                } else {
                    self.versions.contains_key(&f.id)
                }
            })
            .map(|f| {
                (
                    f.id.clone(),
                    self.display(f),
                    self.edits.get(&f.id) == Some(&Edit::Unset),
                )
            })
            .collect();
        if same_session {
            self.edits
                .retain(|id, edit| submission.edits.get(id) != Some(edit));
        }
        self.base = snapshot;
        for (id, raw, unset) in newer {
            let f = schema.settings.iter().find(|f| f.id == id).unwrap();
            let current = f.display(get(&self.base.overrides, &f.path).unwrap_or(&f.default));
            if unset {
                self.edits.insert(id, Edit::Unset);
            } else if raw != current {
                self.edits.insert(id, Edit::Input(raw));
            } else {
                self.edits.remove(&id);
            }
        }
        Ok(())
    }
}
pub(crate) fn get<'a>(table: &'a Table, path: &[String]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let value = table.get(first)?;
    if rest.is_empty() {
        Some(value)
    } else {
        get(value.as_table()?, rest)
    }
}
fn set(table: &mut Table, path: &[String], value: Option<Value>) -> Result<()> {
    let (first, rest) = path
        .split_first()
        .ok_or_else(|| SettingsError::InvalidSchema("empty path".into()))?;
    if rest.is_empty() {
        if let Some(value) = value {
            table.insert(first.clone(), value);
        } else {
            table.remove(first);
        }
    } else {
        if !table.contains_key(first) {
            if value.is_none() {
                return Ok(());
            }
            table.insert(first.clone(), Value::Table(Table::new()));
        }
        let child = table.get_mut(first).unwrap().as_table_mut();
        let Some(child) = child else {
            return Err(SettingsError::InvalidSchema(format!(
                "non-object at {first}"
            )));
        };
        set(child, rest, value)?;
        if child.is_empty() {
            table.remove(first);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_model_validates_schema_drafts_and_submissions_without_store_or_ui() {
        let schema = Schema::parse(include_str!("../tests/fixtures/layout.schema.toml")).unwrap();
        let mut draft = Draft::new(Snapshot::default());
        assert!(!draft.dirty());
        let field = schema.settings[0].clone();
        draft.edit(&field, "false".into());
        assert!(draft.dirty());
        let submission = draft.submit(&schema, "headless-1".into()).unwrap();
        let next = Snapshot {
            revision: 1,
            last_commit_id: "headless-1".into(),
            overrides: submission.overrides.clone(),
        };
        draft.accept(&schema, &submission, next).unwrap();
        assert!(!draft.dirty());
    }
}
