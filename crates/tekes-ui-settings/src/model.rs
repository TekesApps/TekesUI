use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use toml::Value;

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
    pub default: Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub options: Vec<OptionValue>,
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
            .context("unknown Settings item")?;
        let mut selected = self.clone();
        selected.pages[0].label = item.label.clone();
        selected.settings.retain(|f| f.item == item_id);
        selected
            .groups
            .retain(|g| selected.settings.iter().any(|f| f.group == g.id));
        Ok(selected)
    }

    pub fn parse(source: &str) -> Result<Self> {
        let mut schema: Self = toml::from_str(source).context("Settings TOML")?;
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
                ensure!(
                    !item.id.is_empty()
                        && !item.label.trim().is_empty()
                        && item_ids.insert(format!("{}/{}", group.id, item.id)),
                    "invalid/duplicate item {}",
                    item.id
                );
                let mut field_ids = BTreeSet::new();
                for field in &mut item.fields {
                    ensure!(
                        !field.id.is_empty() && field_ids.insert(field.id.clone()),
                        "duplicate item field {}",
                        field.id
                    );
                    field.item = format!("{}/{}", group.id, item.id);
                    field.id = format!("{}/{}", field.item, field.id);
                    field.group = group.id.clone();
                    schema.settings.push(field.clone());
                }
            }
        }
        ensure!(
            schema.schema_version == 1,
            "unsupported Settings schema version"
        );
        ensure!(
            !schema.namespace.trim().is_empty() && !schema.provider.trim().is_empty(),
            "namespace/provider required"
        );
        // The first form has one page; reject rather than silently losing other pages.
        ensure!(
            schema.pages.len() == 1,
            "prototype supports exactly one Settings page"
        );
        ensure!(
            !schema.pages[0].id.is_empty() && !schema.pages[0].label.trim().is_empty(),
            "page ID/label required"
        );
        let mut ids = BTreeSet::new();
        for group in &schema.groups {
            ensure!(
                !group.id.is_empty() && !group.label.trim().is_empty() && ids.insert(&group.id),
                "invalid/duplicate group: {}",
                group.id
            );
            ensure!(
                group.page == schema.pages[0].id,
                "unknown page for group {}",
                group.id
            );
        }
        for group in &schema.groups {
            let mut visiting = BTreeSet::new();
            let mut current = Some(group);
            while let Some(g) = current {
                ensure!(visiting.insert(&g.id), "cyclic Settings group: {}", g.id);
                current = if let Some(parent) = &g.parent {
                    Some(
                        schema
                            .groups
                            .iter()
                            .find(|g| &g.id == parent)
                            .with_context(|| format!("unknown parent {parent}"))?,
                    )
                } else {
                    None
                };
            }
        }
        let mut field_ids = BTreeSet::new();
        let mut paths: Vec<&Vec<String>> = Vec::new();
        for field in &schema.settings {
            ensure!(
                !field.id.is_empty()
                    && !field.label.trim().is_empty()
                    && field_ids.insert(&field.id),
                "invalid/duplicate field: {}",
                field.id
            );
            ensure!(ids.contains(&field.group), "unknown group: {}", field.group);
            ensure!(
                !field.path.is_empty()
                    && field.path.iter().all(|p| !p.is_empty())
                    && field.path[0] != "_tekes",
                "invalid/reserved path: {}",
                field.id
            );
            ensure!(
                paths
                    .iter()
                    .all(|p| !p.starts_with(&field.path) && !field.path.starts_with(p)),
                "overlapping Settings path: {}",
                field.id
            );
            paths.push(&field.path);
            ensure!(
                field.min.is_none_or(f64::is_finite) && field.max.is_none_or(f64::is_finite),
                "invalid range"
            );
            ensure!(
                field.min.zip(field.max).is_none_or(|(a, b)| a <= b),
                "reversed range"
            );
            if field.step.is_some() || !field.suffix.is_empty() {
                ensure!(
                    matches!(field.kind, FieldType::Integer | FieldType::Decimal),
                    "numeric format on nonnumeric field"
                );
                ensure!(
                    field.min.is_some() && field.max.is_some(),
                    "numeric control requires explicit bounds"
                );
                ensure!(
                    field.step.is_none_or(|s| s.is_finite() && s > 0.),
                    "invalid numeric step"
                );
                ensure!(
                    field.kind != FieldType::Integer || field.step.is_none_or(|s| s.fract() == 0.),
                    "integer step must be integral"
                );
            }
            let mut options = BTreeSet::new();
            ensure!(
                field.options.iter().all(|o| !o.value.is_empty()
                    && !o.label.trim().is_empty()
                    && options.insert(&o.value)),
                "invalid/duplicate option: {}",
                field.id
            );
            field
                .validate(&field.default)
                .with_context(|| format!("default {}", field.id))?;
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
        ensure!(valid, "{}: invalid {:?} value", self.label, self.kind);
        let number = value
            .as_float()
            .or_else(|| value.as_integer().map(|v| v as f64));
        if let Some(n) = number {
            ensure!(
                n.is_finite() && self.min.is_none_or(|m| n >= m) && self.max.is_none_or(|m| n <= m),
                "{}: value outside allowed range",
                self.label
            );
        }
        Ok(())
    }
    pub fn parse_input(&self, raw: &str) -> Result<Value> {
        let value = match self.kind {
            FieldType::String | FieldType::Enum => Value::String(raw.into()),
            FieldType::Boolean => Value::Boolean(raw.parse().context("expected boolean")?),
            FieldType::Integer => Value::Integer(
                raw.trim()
                    .parse()
                    .with_context(|| format!("{}: expected integer", self.label))?,
            ),
            FieldType::Decimal => Value::Float(
                raw.trim()
                    .parse()
                    .with_context(|| format!("{}: expected number", self.label))?,
            ),
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
    pub overrides: toml::Table,
}
// JSON alone cannot preserve TOML datetime or non-finite float variants. The
// typed snapshot envelope carries its unknown-key payload as lossless TOML.
mod snapshot_table {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &toml::Table, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&toml::to_string(value).map_err(serde::ser::Error::custom)?)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<toml::Table, D::Error> {
        let source = String::deserialize(deserializer)?;
        toml::from_str(&source).map_err(serde::de::Error::custom)
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
    pub overrides: toml::Table,
    edits: BTreeMap<String, Edit>,
    versions: BTreeMap<String, u64>,
}
impl Submission {
    /// Restore a complete saved snapshot through the same single-writer Provider.
    /// History still requires its own durable confirmation after this write.
    pub fn replacement(before: &Snapshot, after: &Snapshot) -> Result<Self> {
        ensure!(
            after.revision
                == before
                    .revision
                    .checked_add(1)
                    .context("revision exhausted")?
                && !after.last_commit_id.is_empty(),
            "invalid replacement identity"
        );
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
        ensure!(self.dirty(), "no changes to apply");
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
        ensure!(
            snapshot.last_commit_id == submission.commit_id
                && snapshot.revision == submission.expected_revision + 1
                && snapshot.overrides == submission.overrides,
            "mismatched Settings receipt"
        );
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
pub(crate) fn get<'a>(table: &'a toml::Table, path: &[String]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let value = table.get(first)?;
    if rest.is_empty() {
        Some(value)
    } else {
        get(value.as_table()?, rest)
    }
}
fn set(table: &mut toml::Table, path: &[String], value: Option<Value>) -> Result<()> {
    let (first, rest) = path.split_first().context("empty path")?;
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
            table.insert(first.clone(), Value::Table(toml::Table::new()));
        }
        let child = table.get_mut(first).unwrap().as_table_mut();
        let Some(child) = child else {
            bail!("non-object at {first}");
        };
        set(child, rest, value)?;
        if child.is_empty() {
            table.remove(first);
        }
    }
    Ok(())
}
