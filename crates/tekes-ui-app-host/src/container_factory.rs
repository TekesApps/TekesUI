//! Explicit consumer-owned factories. Worker factories remain compiled into the app.
use crate::{Container, ContainerOptions, LocalContainer, ModuleDefinition, ProcessContainer};
use anyhow::Result;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerMultiplicity {
    Single,
    Multiple,
}

type LocalFactory = Rc<dyn Fn(&str) -> Result<LocalContainer>>;
#[derive(Clone)]
enum Factory {
    Local(LocalFactory),
    Process(ModuleDefinition, ContainerOptions),
}
/// A registered creatable type, separate from every stable instance identity.
#[derive(Clone)]
pub struct ContainerFactory {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) icon: String,
    pub(crate) multiplicity: ContainerMultiplicity,
    factory: Factory,
}
impl ContainerFactory {
    pub fn local(
        id: impl Into<String>,
        title: impl Into<String>,
        multiplicity: ContainerMultiplicity,
        create: impl Fn(&str) -> Result<LocalContainer> + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: "tekes-file-generic".into(),
            multiplicity,
            factory: Factory::Local(Rc::new(create)),
        }
    }
    pub fn process(
        id: impl Into<String>,
        title: impl Into<String>,
        multiplicity: ContainerMultiplicity,
        definition: ModuleDefinition,
        options: ContainerOptions,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: options
                .switcher_icon
                .clone()
                .unwrap_or_else(|| "tekes-file-generic".into()),
            multiplicity,
            factory: Factory::Process(definition, options),
        }
    }
    pub fn switcher_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self
    }
    pub(crate) fn definition(&self) -> Option<ModuleDefinition> {
        match self.factory {
            Factory::Process(definition, _) => Some(definition),
            _ => None,
        }
    }
    pub(crate) fn create(&self, id: &str, ordinal: u64) -> Result<Container> {
        let mut container = match &self.factory {
            Factory::Local(create) => Container::Local(create(id)?),
            Factory::Process(definition, options) => ProcessContainer::new(*definition)
                .instance_id(id)
                .options(options.clone())
                .into(),
        };
        anyhow::ensure!(
            container.id() == id,
            "factory {} returned a different instance ID",
            self.id
        );
        let options = match &mut container {
            Container::Local(c) => &mut c.options,
            Container::Process(c) => &mut c.options,
        };
        if options.title.is_none() {
            options.title = Some(if self.multiplicity == ContainerMultiplicity::Multiple {
                format!("{} {ordinal}", self.title)
            } else {
                self.title.clone()
            });
        }
        if options.switcher_icon.is_none() {
            options.switcher_icon = Some(self.icon.clone());
        }
        Ok(container)
    }
}
pub(crate) fn validate(factories: &[ContainerFactory]) -> Result<()> {
    let mut ids = std::collections::HashSet::new();
    for factory in factories {
        if let Some(definition) = factory.definition() {
            crate::validate(&crate::AppProfile {
                modules: vec![definition],
            })?;
        }
        anyhow::ensure!(
            !factory.id.is_empty()
                && factory
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
            "invalid container factory ID"
        );
        anyhow::ensure!(
            ids.insert(&factory.id),
            "duplicate container factory ID: {}",
            factory.id
        );
        anyhow::ensure!(
            !factory.title.trim().is_empty(),
            "empty container factory title"
        );
        anyhow::ensure!(
            tekes_ui_icons::Glyph::named(&factory.icon).is_some(),
            "unknown container factory icon"
        );
    }
    Ok(())
}

pub(crate) struct RegisteredFactory {
    pub(crate) definition: ContainerFactory,
    pub(crate) next_instance: u64,
    pub(crate) single: Option<String>,
}
