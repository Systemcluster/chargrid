extern crate prototty;
extern crate cgmath;
extern crate terminal_colour;

use cgmath::Vector2;
use prototty::*;
use terminal_colour::colours;

const ESCAPE: char = '\u{1b}';
const ETX: char = '\u{3}';
const WIDTH: u16 = 10;
const HEIGHT: u16 = 10;

fn main() {

    let root_div = AbsDiv::new((20, 10)).into_handle();
    let root = ElementHandle::from(root_div.clone());

    let container = AbsDiv::new((18, 8)).into_handle();
    root_div.insert("container", container.clone(), (1, 1), None);

    container.insert("title", Text::new("Title", (6, 1)), (0, 0), None);

    let canvas = Canvas::new((WIDTH, HEIGHT)).into_handle();
    container.insert("canvas", canvas.clone(), (0, 1), None);

    let mut buffer = canvas.make_buffer();

    let mut ctx = Context::new().unwrap();

    let mut coord = Vector2::new(0, 0);

    loop {

        for j in 0..HEIGHT as i16 {
            for i in 0..WIDTH as i16 {
                let grid_coord = Vector2::new(i, j);
                let cell = buffer.get_mut(grid_coord).unwrap();
                if grid_coord == coord {
                    cell.background_colour = colours::RED;
                } else {
                    cell.background_colour = colours::WHITE;
                }
            }
        }

        canvas.swap_buffer(&mut buffer).unwrap();
        ctx.render(&root).unwrap();

        let direction = match ctx.wait_input().unwrap() {
            Input::Char(ESCAPE) | Input::Char(ETX) => break,
            Input::Left => Vector2::new(-1, 0),
            Input::Right => Vector2::new(1, 0),
            Input::Up => Vector2::new(0, -1),
            Input::Down => Vector2::new(0, 1),
            _ => continue,
        };

        let new_coord = coord + direction;
        if new_coord.x < 0 || new_coord.y < 0 || new_coord.x >= WIDTH as i16 || new_coord.y >= HEIGHT as i16 {
            continue;
        }

        coord = new_coord;
    }
}
