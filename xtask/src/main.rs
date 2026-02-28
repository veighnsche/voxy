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
    Fixtures(FixturesArgs),
    Gui(GuiArgs),
}

#[derive(Debug, Args)]
struct FixturesArgs {
    #[command(subcommand)]
    command: FixturesCommand,
}

#[derive(Debug, Subcommand)]
enum FixturesCommand {
    FetchAudio(tasks::fixtures::fetch_audio::FetchAudioArgs),
    VerifyAudio(tasks::fixtures::verify_audio::VerifyAudioArgs),
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
    SttE2e(tasks::gui::stt_e2e::SttE2eArgs),
    VisibilitySmoke(tasks::gui::visibility_smoke::VisibilitySmokeArgs),
    VisibilityToggleFlow(tasks::gui::visibility_toggle_flow::VisibilityToggleFlowArgs),
    VisibilityWindowGuard(tasks::gui::visibility_window_guard::VisibilityWindowGuardArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Fixtures(fixtures) => match fixtures.command {
            FixturesCommand::FetchAudio(args) => tasks::fixtures::fetch_audio::run(args),
            FixturesCommand::VerifyAudio(args) => tasks::fixtures::verify_audio::run(args),
        },
        Command::Gui(gui) => match gui.command {
            GuiCommand::DragMathSim(args) => tasks::gui::drag_math_sim::run(args),
            GuiCommand::Lifecycle(args) => tasks::gui::lifecycle::run(args),
            GuiCommand::ResetFlow(args) => tasks::gui::reset_flow::run(args),
            GuiCommand::Smoke(args) => tasks::gui::smoke::run(args),
            GuiCommand::SttE2e(args) => tasks::gui::stt_e2e::run(args),
            GuiCommand::VisibilitySmoke(args) => tasks::gui::visibility_smoke::run(args),
            GuiCommand::VisibilityToggleFlow(args) => tasks::gui::visibility_toggle_flow::run(args),
            GuiCommand::VisibilityWindowGuard(args) => {
                tasks::gui::visibility_window_guard::run(args)
            }
        },
    }
}
