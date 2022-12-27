mod receiving_side {
    use animation_lang::program::Program;
    use animation_lang::vm::{VMConfig, VMStateConfig, VM};
    use colored::Colorize;

    pub fn animation_loop(bin_prog: Vec<u8>) {
        // Initialize VM with 10 leds with default config
        let vm = VM::new(10, VMConfig::default());
        // Start program in VM
        let vm_state = vm.start(Program::from_binary(bin_prog), VMStateConfig::default());

        // print first 10 frames into terminal
        for (i, frame_res) in vm_state.take(10).enumerate() {
            let frame = frame_res.unwrap(); // Could have encountered runtime error

            // Print frame in terminal
            print!("frame #{}: ", i);
            for pixel in frame {
                print!("{}", "■".truecolor(pixel.r, pixel.g, pixel.b));
            }
            println!();
        }
    }
}

mod sending_side {
    use animation_lang::compiler::FromSource;
    use animation_lang::program::Program;

    pub fn compile_example_prog(source_code: &str) -> Vec<u8> {
        Program::from_source(source_code).unwrap().code().to_vec()
    }
}

const EXAMPLE_PROG: &str = "
loop {
  for(i=get_length) {
    // Blank led strip
    for(n=get_length) {
      set_pixel(n-1,0,0,0);
    };

    set_pixel(get_length-i,255,255,255); // Enable next pixel (0, 1, 2...)

    blit; // Yield frame
  };
}";

fn main() {
    use receiving_side::animation_loop;
    use sending_side::compile_example_prog;

    let compiled_prog = compile_example_prog(EXAMPLE_PROG);

    animation_loop(compiled_prog);
}
