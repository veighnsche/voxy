mod tasks;
mod workspace;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Workspace automation tasks for Voxy")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Gui(GuiArgs),
}

#[derive(Debug, Args)]
struct GuiArgs {
    #[command(subcommand)]
    command: GuiCommand,
}

#[derive(Debug, Subcommand)]
enum GuiCommand {
    DragMathSim(tasks::gui::drag_math_sim::DragMathSimArgs),
    Lifecycle(tasks::gui::lifecycle::LifecycleArgs),
    ResetFlow(tasks::gui::reset_flow::ResetFlowArgs),
    Smoke(tasks::gui::smoke::SmokeArgs),
    VisibilitySmoke(tasks::gui::visibility_smoke::VisibilitySmokeArgs),
    VisibilityToggleFlow(tasks::gui::visibility_toggle_flow::VisibilityToggleFlowArgs),
    VisibilityWindowGuard(tasks::gui::visibility_window_guard::VisibilityWindowGuardArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Gui(gui) => match gui.command {
            GuiCommand::DragMathSim(args) => tasks::gui::drag_math_sim::run(args),
            GuiCommand::Lifecycle(args) => tasks::gui::lifecycle::run(args),
            GuiCommand::ResetFlow(args) => tasks::gui::reset_flow::run(args),
            GuiCommand::Smoke(args) => tasks::gui::smoke::run(args),
            GuiCommand::VisibilitySmoke(args) => tasks::gui::visibility_smoke::run(args),
            GuiCommand::VisibilityToggleFlow(args) => tasks::gui::visibility_toggle_flow::run(args),
            GuiCommand::VisibilityWindowGuard(args) => {
                tasks::gui::visibility_window_guard::run(args)
            }
        },
    }
}
