use std::process::abort;

use itertools::Itertools;

use gears::{AnyMatch, create_selected_output_builders, output_builder_from_str};
use gears::cli::Game;
use gears::games::{Board, OutputList, RectangularBoard, RectangularCoordinates};
use gears::games::chess::Chessboard;
use gears::games::mnk::MNKBoard;
use gears::general::common::{Res, select_name_dyn};
use gears::general::common::Description::WithDescription;
use gears::output::{normal_outputs, required_outputs};

use crate::cli::{CommandLineArgs, HumanArgs, parse_cli, PlayerArgs};
use crate::play::player::PlayerBuilder;
use crate::play::ugi_client::RunClient;
use crate::ui::{InputBuilder, InputList};
use crate::ui::text_input::TextInputBuilder;

pub mod cli;
pub mod play;
pub mod ui;

fn main() {
    if let Err(err) = run_program() {
        eprintln!("{err}");
        abort();
    }
}

pub fn text_based_inputs<B: Board>() -> InputList<B> {
    vec![
        Box::new(TextInputBuilder::default()),
        // TODO: Add SPRT input
    ]
}

pub fn required_uis<B: Board>() -> (OutputList<B>, InputList<B>) {
    (required_outputs(), text_based_inputs())
}

pub fn normal_uis<B: RectangularBoard>() -> (OutputList<B>, InputList<B>)
where
    <B as Board>::Coordinates: RectangularCoordinates,
{
    (normal_outputs(), text_based_inputs()) // TODO: Add additional interactive uis, like a GUI
}

fn list_chess_uis() -> (OutputList<Chessboard>, InputList<Chessboard>) {
    normal_uis::<Chessboard>()
}

fn list_mnk_uis() -> (OutputList<MNKBoard>, InputList<MNKBoard>) {
    normal_uis::<MNKBoard>()
}

pub fn create_input_from_str<B: Board>(
    name: &str,
    opts: &str,
    list: &[Box<dyn InputBuilder<B>>],
) -> Res<Box<dyn InputBuilder<B>>> {
    let mut ui_builder = dyn_clone::clone_box(select_name_dyn(
        name,
        list,
        "input",
        B::game_name(),
        WithDescription,
    )?);
    ui_builder.set_option(opts)?;
    Ok(ui_builder)
}

pub fn map_ui_to_input_and_output(ui: &str) -> (&str, &str) {
    match ui {
        "text" => ("text", "unicode"),
        "gui" => todo!(),
        "sprt" => (todo!(), "none"),
        x => (x, x),
    }
}

// TODO: Use #[cfg()] to conditionally include `motors` and its engines

pub fn create_match(args: CommandLineArgs) -> Res<AnyMatch> {
    // match mode {
    //     Gui(options) => {
    match args.game {
        Game::Chess => create_gui_match_for_game(args, list_chess_uis()),
        Game::Mnk => create_gui_match_for_game(args, list_mnk_uis()),
    }
    // }
    //     mode => {
    //         #[cfg(feature = "motors")]
    //         return motors::create_match(game, mode);
    //         return Err(format!("The command line argument '{}' can't be used with this version of `monitors` because it doesn't include any engines.", mode));
    //     }
    // }
}

pub fn create_gui_match_for_game<B: Board>(
    mut args: CommandLineArgs,
    uis: (OutputList<B>, InputList<B>),
) -> Res<AnyMatch> {
    while args.players.len() < 2 {
        args.players.push(PlayerArgs::Human(HumanArgs::default()));
    }

    let (input_name, output_name) = map_ui_to_input_and_output(&args.ui);
    let mut outputs = create_selected_output_builders(&args.additional_outputs, &uis.0)?;
    let output = output_builder_from_str(output_name, &uis.0)?;
    outputs.insert(0, output);
    if args.debug && !outputs.iter().any(|x| x.short_name() == "logger") {
        outputs.push(output_builder_from_str("logger", &uis.0)?);
    }
    let input = create_input_from_str(input_name, "", &uis.1)?.build();
    let run_client = Box::new(RunClient::create(input, uis.0, &args)?);
    {
        let mut client_mutex = run_client.client.lock().unwrap();
        client_mutex.state.debug = args.debug;
        for output in outputs {
            client_mutex.add_output(output);
        }
    }
    let client = run_client.client.clone();
    let mut builders = args.players.into_iter().map(|p| PlayerBuilder::new(p));
    for mut builder in builders {
        builder.build(client.clone())?;
    }

    Ok(run_client)
}

pub fn run_program() -> Res<()> {
    let args = parse_cli().map_err(|err| format!("Error parsing command line arguments: {err}"))?;

    let mut the_match =
        create_match(args).map_err(|err| format!("Couldn't start the client: {err}"))?;
    the_match.run();
    Ok(())
}
