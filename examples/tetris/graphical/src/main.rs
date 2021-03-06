use chargrid_graphical::*;
use tetris_app::TetrisApp;

fn main() {
    env_logger::init();
    let context = Context::new(ContextDescriptor {
        font_bytes: FontBytes {
            normal: include_bytes!("./fonts/PxPlus_IBM_CGAthin.ttf").to_vec(),
            bold: include_bytes!("./fonts/PxPlus_IBM_CGA.ttf").to_vec(),
        },
        title: "Tetris".to_string(),
        window_dimensions: Dimensions {
            width: 640.,
            height: 480.,
        },
        cell_dimensions: Dimensions {
            width: 32.,
            height: 32.,
        },
        font_dimensions: Dimensions {
            width: 32.,
            height: 32.,
        },
        font_source_dimensions: Dimensions {
            width: 32.,
            height: 32.,
        },
        underline_width: 0.1,
        underline_top_offset: 0.8,
        resizable: false,
    })
    .unwrap();
    let app = TetrisApp::new(rand::thread_rng());
    context.run_app(app);
}
