use renderer::window::run;

mod renderer;
mod game;

fn main() {
    pollster::block_on(run());
}
