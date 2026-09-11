use crate::actions::LoadInstanceAction;
use crate::commands::{self, Command};
use crate::error::Result;
use crate::models::InstanceName;
use crate::view::Console;
use clap::Parser;
use std::sync::Arc;

/// Rename a VM instance
///
/// Examples:
///
///   Rename the VM instance 'noble' in 'ubuntu':
///   $ cubic rename noble ubuntu
///
#[derive(Parser)]
#[clap(verbatim_doc_comment)]
pub struct RenameCommand {
    /// Name of the virtual machine instance to rename
    old_name: InstanceName,
    /// New name of the virtual machine instance
    new_name: InstanceName,
}

impl Command for RenameCommand {
    async fn run(&self, console: &Arc<Console>, context: &commands::Context) -> Result<u8> {
        let instance_store = context.get_instance_store();

        instance_store.rename(
            &mut LoadInstanceAction::new().run(context, console, self.old_name.as_str())?,
            self.new_name.as_str(),
        )?;
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::instance::InstanceStoreMock;
    use crate::models::{Environment, Instance, UserName};
    use crate::platform::SystemMock;
    use std::str::FromStr;
    use std::sync::Arc;

    fn build_context(instances: Vec<Instance>) -> commands::Context {
        let env = Environment::new(
            UserName::from_str("cubic").unwrap(),
            String::new(),
            String::new(),
        );
        commands::Context::new(
            Arc::new(SystemMock::new()),
            env,
            Box::new(InstanceStoreMock::new(instances)),
        )
    }

    #[tokio::test]
    async fn test_rename_rejects_unknown_instance() {
        let system = SystemMock::new();
        let console = &Console::new(Arc::new(system));
        let context = build_context(Vec::new());

        let result = RenameCommand {
            old_name: InstanceName::from_str("missing").unwrap(),
            new_name: InstanceName::from_str("newname").unwrap(),
        }
        .run(console, &context)
        .await;

        assert!(matches!(
            result,
            Err(Error::UnknownInstance(ref name)) if name == "missing"
        ));
    }

    #[tokio::test]
    async fn test_rename_delegates_to_store() {
        let system = SystemMock::new();
        let console = &Console::new(Arc::new(system));
        let context = build_context(vec![Instance {
            name: "test".to_string(),
            ..Instance::default()
        }]);

        let result = RenameCommand {
            old_name: InstanceName::from_str("test").unwrap(),
            new_name: InstanceName::from_str("newname").unwrap(),
        }
        .run(console, &context)
        .await;

        assert!(result.is_ok());
    }
}
