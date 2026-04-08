use crate::commands::CommandArgs;

pub(crate) struct MainPrGate {
    args: CommandArgs,
}

impl MainPrGate {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self { args }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let head_ref = self.args.extract_flag_value_or("--head-ref", "");

        if head_ref == "dev" || head_ref.starts_with("hotfix/") {
            println!("OK: Branch '{head_ref}' is allowed to merge into main.");
            return Ok(());
        }

        Err(format!(
            "Only 'dev' or 'hotfix/*' branches can merge into main. Received: '{head_ref}'."
        ))
    }
}
