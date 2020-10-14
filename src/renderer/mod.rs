mod scancode;

use glium::glutin;
use pixel_canvas::{
    canvas::CanvasInfo,
    input::{Event, WindowEvent},
    Canvas, Color,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const FRAME_SELECT: usize = 0x200604;
const FRAME_0: usize = 0;
const FRAME_1: usize = 0x100000;
// const FRAME_SIZE: usize = WIDTH * HEIGHT;

struct MyState {
    key_buffer: VecDeque<u8>,
}

impl MyState {
    fn new() -> Self {
        Self {
            key_buffer: VecDeque::new(),
        }
    }

    fn handle_input(_info: &CanvasInfo, state: &mut MyState, event: &Event<()>) -> bool {
        match event {
            // Match a keypress with scancode "key"
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            glutin::event::KeyboardInput {
                                state: glutin::event::ElementState::Pressed,
                                scancode: key,
                                ..
                            },
                        is_synthetic: false,
                        ..
                    },
                ..
            } => {
                dbg!(scancode::to_ascii(*key) as char);
                state.key_buffer.push_back(scancode::to_ascii(*key));
                true
            }

            _ => false,
        }
    }
}

// TODO: change the color format in pixel-canvas to ClientFormat::U8
fn mmio_color_to_rgb(x: u8) -> Color {
    let r = x & 0b111;
    let g = (x >> 3) & 0b111;
    let b = x >> 6;
    Color {
        r: r * 32,
        g: g * 32,
        b: b * 64,
    }
}

pub fn init(mmio: Arc<Mutex<Vec<u8>>>) {
    let canvas = Canvas::new(2 * WIDTH, 2 * HEIGHT)
        .title("FPGRARS")
        .state(MyState::new())
        .input(MyState::handle_input);

    #[cfg(debug_assertions)]
    let canvas = canvas.show_ms(true);

    canvas.render(move |_state, image| {
        let mmio = mmio.lock().unwrap();

        let frame = mmio[FRAME_SELECT];
        let start = if frame == 0 { FRAME_0 } else { FRAME_1 };

        // Draw each MMIO pixel as a 2x2 square
        for (y, row) in image.chunks_mut(2 * WIDTH).enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                let (x, y) = (x / 2, y / 2);
                let index = start + y * WIDTH + x;

                let col = if cfg!(debug_assertions) {
                    *mmio
                        .get(index)
                        .expect("Out of bound access to the video memory!")
                } else {
                    unsafe { *mmio.get_unchecked(index) }
                };

                *pixel = mmio_color_to_rgb(col);
            }
        }

        // Alternative, possibly slower, implementation:

        // let mut set = move |i, col| {
        //     if cfg!(debug_assertions) {
        //         *image
        //             .get_mut(i)
        //             .expect("Out of bounds access to the video memory!") = mmio_color_to_rgb(col);
        //     } else {
        //         unsafe {
        //             *image.get_unchecked_mut(i) = mmio_color_to_rgb(col);
        //         }
        //     }
        // };

        // for i in 0..FRAME_SIZE {
        //     let col = mmio[i + start];

        //     // 0xC7 is "transparent"
        //     if col != 0xC7 {
        //         // Don't ask
        //         // TODO: if this is too slow, we can try filling in line by line,
        //         // as every other line is just a copy of the one above it
        //         {
        //             set((i % WIDTH) * 2 + (i / WIDTH) * WIDTH * 4, col);
        //             set(1 + (i % WIDTH) * 2 + (i / WIDTH) * WIDTH * 4, col);
        //             set((i % WIDTH) * 2 + (i / WIDTH) * WIDTH * 4 + 2 * WIDTH, col);
        //             set(
        //                 1 + (i % WIDTH) * 2 + (i / WIDTH) * WIDTH * 4 + 2 * WIDTH,
        //                 col,
        //             );
        //         }
        //     }
        // }
    });
}
