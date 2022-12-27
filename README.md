# Animation-lang library

animation-lang library is a compiler and vm couple written in Rust for primitive self-made programming language,
called `animation-language`. Its goal is to provide method for creating animations on addressable led
strips on the fly, without requiring re-flash of microcontroller which drives led strip.

## Table of contents

1. [Animation-lang syntax](#animation-language-syntax)
2. [Virtual Machine details](#virtual-machine-details)
    1. [Instruction set](#instructions-set--p-codes--)
3. [Library usage](#library-usage-example)
    1. [Basic example](#basic)
    2. [More examples](#more-examples)
4. [Licence](#license)

## Animation-language syntax

Syntax of this programming language is mostly `c`-like,
each expression and some statements should be followed by `;`

### Literals - decimal or hexadecimals numbers

Just numbers like `12` or `0x2F`, minimal value: 0, maxim: 2^32-1

### Expressions

basic: `+` `-` `*` `/` `%`

logical: `==` `!=` `>` `>=` `<` `<=`

bitwise: `<<` `>>` `&` `|` `^` `!`

### Constant variables

Assign value of expression to named constants

```
some_var1 = 10 + 20;
some_var2 = 10 << 8;
```

Redefining constant is forbidden, until previous one with such name leaves scope.

### Statements

#### forever `loop`

Code in such loop will be executed forever, no break method provided.

```
loop {
   ...
}
```

#### `for` loop

Loop with limited number of iterations, counts `var` down from expression to 1 (inclusive).

```
for(var=5) {
  // will repeat block 5 times with, n = 5,4,3,2,1
};
```

#### `if` and `if else`

```
if(n==0) {
  ...
}
```

```
if(n==0) {
   ...
} else {
   ...
}
```

### Special expressions

#### _get_length_

returns current length of led strip, e.g:

```
smth = get_length;
```

will place into constant smth current quantity of leds in strip.

#### _set_pixel(i, r, g, b)_

sets color of pixel in buffer, e.g:

```
for(i=get_length) {
  set_pixel(i-1, 255, 255, 255);
};
```

will set all pixels in buffer to white

#### _get_pixel(i)_

returns color of pixel in internal buffer at provided index

```
color = get_pixel(25);
r = color & 0xFF;
g = (color >> 8) & 0xFF;
b = (color >> 16) & 0xFF;
```

#### _random(n)_

returns random integer in range `0`...`n`, excluding `n`

```
for(i=random(10)) {
   ...
}
```

will repeat loop body from `0` upto `9` times.

#### _get_wall_time_

returns number of seconds since UNIX_EPOCH

```
min = get_wall_time % 360;
sec = get_wall_time % 60;
```

#### _get_precise_time_

returns number of millis since program start time

```
secs_passed = get_precise_time / 1000;
```

#### _blit_

yields vm internal led buffer

```
blit;
```

typically used to give caller new frame to display

### Compiler intrinsics

### _rgb(r_var, g_var, b_var)_

translates to `(r & 0xFF) | (g & 0xFF) << 8 | (b & 0xFF) << 16`

```
r = 20;
g = 10;
b = 210;
combined_color = rgb(r,g,b);
```

### _red(c)_

translates to `c & 0xFF`

```
color = get_pixel(5);
r = red(color);
```

### _green(c)_

translates to `(c >> 8) & 0xFF`

```
color = get_pixel(5);
g = green(color);
```

### _blue(c)_

translates to `(c >> 16) & 0xFF`

```
color = get_pixel(5);
b = blue(color);
```

### _clamp(val, min, max)_

translates to:

```
if (val > max) {
   max
} else {
   if (val < min) {
      min
   } else {
      val
   }
}
```

### Comments

#### Single line comment

everything after `//` on the same line will be ignored by compiler

```

// comment on separate line
i = 10; // comment on same line

```

#### Multi line comment

everything after `/*` and before `*/` will be ignored by compiler

```

/* one line */
b = 0; /* same line,
but multiline!
*/

```

### Example programs

Example of programs in _animation-language_ can be found in `example_progs` directory.

## Virtual Machine details

Provided runtime implemented though stack based p-code machine, with fairly small instruction set.

Each byte in output program is ether data or instruction, depends on previous instruction.
Each instruction is a single byte, where the first nibble (4 bits) determine the instruction type and the next nibble a
variant.

Virtual Machine states has:

* `mem` - byte array for string program bytes.
* `stack` - runtime stack used by program for storing 32bit integers.
* `pixel_buf` - internal pixel buffer for storing colors in format `0xRRGGBB00` (32bit integers).

### Instructions set (p-codes):

<table>
    <thead>
        <tr>
            <th>Instruction type</th>
            <th>Instruction variant</th>
            <th>Description</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td rowspan=1><code>POP</code></td>
            <td><code>0x00-0x0F</code></td>
            <td>pop given number of integers from <code>stack</code></td>
        </tr>
        <tr>
            <td rowspan=1><code>PUSHB</code></td>
            <td><code>0x00-0x0F</code></td>
            <td>push given number of bytes on <code>stack</code> from <code>mem</code>, each byte will take 32bit</td>
        </tr>
        <tr>
            <td rowspan=1><code>PUSHI</code></td>
            <td><code>0x00-0x0F</code></td>
            <td>push given number of 4 bytes blocks on <code>stack</code> from <code>mem</code>, bytes would be packed</td>
        </tr>
        <tr>
            <td rowspan=1><code>PEEK</code></td>
            <td><code>0x00-0x0F</code></td>
            <td>push integer at <code>stack[index]</code> on <code>stack</code></td>
        </tr>
        <tr>
            <td rowspan=1><code>JMP</code></td>
            <td><code>ignored</code></td>
            <td>extracts address from next 2 bytes in <code>mem</code> and jumps to it</td>
        </tr>
        <tr>
            <td rowspan=1><code>JZ</code></td>
            <td><code>ignored</code></td>
            <td>same sa <code>JMP</code>, but jump only if last value on <code>stack</code> is 0</td>
        </tr>
         <tr>
            <td rowspan=1><code>JNZ</code></td>
            <td><code>ignored</code></td>
            <td>same sa <code>JMP</code>, but jump only if last value on <code>stack</code> is <strong>not</strong> 0</td>
        </tr>
        <tr>
            <td rowspan=5><code>UNARY</code></td>
            <td><code>INC</code></td>
            <td>increment last value on <code>stack</code> by 1</td>
        </tr>
        <tr>
            <td><code>DEC</code></td>
            <td>decrement last value on <code>stack</code> by 1</td>
        </tr>
        <tr>
            <td rowspan=1><code>NOT</code></td>
            <td>decrement last value on <code>stack</code> by 1</td>
        </tr>
        <tr>
            <td><code>SHL8</code></td>
            <td>perform bitwise left shift operation on last value in <code>stack</code> by 8 bits</td>
        </tr> 
        <tr>
            <td><code>SHR8</code></td>
            <td>perform bitwise right shift operation on last value in <code>stack</code> by 8 bits</td>
        </tr>
        <tr>
            <td rowspan=16>
               <code>BINARY</code> - pop last two values from <code>stack</code>:
               <code>rhs</code> and <code>lhs</code>, perform operation and push result on <code>stack</code>
            </td>
            <td><code>ADD</code></td>
            <td>sum <code>lhs</code> with <code>rhs</code></td>
        </tr>
        <tr>
            <td><code>SUB</code></td>
            <td>subtract <code>rhs</code> from <code>lhs</code></td>
        </tr>
        <tr>
            <td rowspan=1><code>DIV</code></td>
            <td>divide <code>lhs</code> by <code>lhs</code></td>
        </tr>
        <tr>
            <td><code>MUL</code></td>
            <td>multiply <code>lhs</code> by <code>lhs</code></td>
        </tr> 
        <tr>
            <td><code>MOD</code></td>
            <td>calculate reminder after dividing <code>lhs</code> by <code>lhs</code></td>
        </tr>
        <tr>
            <td><code>AND</code></td>
            <td>perform logical <strong>and</strong> on <code>lhs</code> and <code>lhs</code></td>
        </tr>
        <tr>
            <td><code>OR</code></td>
            <td>perform logical <strong>or</strong> on <code>lhs</code> and <code>lhs</code></td>
        </tr>
        <tr>
            <td><code>XOR</code></td>
            <td>perform bitwise <strong>xor</strong> on <code>lhs</code> and <code>lhs</code></td>
        </tr>
        <tr>
            <td><code>GT</code></td>
            <td><code>1</code> if <code>lhs</code> > <code>rhs</code>, otherwise <code>0</code></td>
        </tr>
        <tr>
            <td><code>GTE</code></td>
            <td><code>1</code> if <code>lhs</code> >= <code>rhs</code>, otherwise <code>0</code></td>
        </tr>
        <tr>
            <td><code>LT</code></td>
            <td><code>1</code> if <code>lhs</code> < <code>rhs</code>, otherwise <code>0</code></td>
        </tr>
        <tr>
            <td><code>LTE</code></td>
            <td><code>1</code> if <code>lhs</code> <= <code>rhs</code>, otherwise <code>0</code></td>
        </tr>
        <tr>
            <td><code>EQ</code></td>
            <td><code>1</code> if <code>lhs</code> == <code>rhs</code>, otherwise <code>0</code></td>
        </tr>
        <tr>
            <td><code>NEQ</code></td>
            <td><code>1</code> if <code>lhs</code> != <code>rhs</code>, otherwise <code>0</code></td>
        </tr>
        <tr>
            <td><code>SHL</code></td>
            <td>perform bitwise left shift <code>lhs</code> by <code>rhs</code> bits</td>
        </tr>
        <tr>
            <td><code>SHR</code></td>
            <td>perform bitwise right shift <code>lhs</code> by <code>rhs</code> bits</td>
        </tr>
        <tr>
            <td rowspan=7><code>USER</code></td>
            <td><code>GET_LENGTH</code></td>
            <td>push led strip length on <code>stack</code></td>
        </tr>
        <tr>
            <td><code>GET_WALL_TIME</code></td>
            <td>push seconds since <code>UNIX_EPOCH</code> on <code>stack</code></td>
        </tr>
        <tr>
            <td rowspan=1><code>GET_PRECISE_TIME</code></td>
            <td>push milliseconds since program start on <code>stack</code></td>
        </tr>
        <tr>
            <td><code>SET_PIXEL</code></td>
            <td>pop int from <code>stack</code> as <code>color</code>, then peek int from <code>stack</code> as <code>index</code>, and set <code>pixel_buf[index]</code> to <code>color</code></td>
        </tr> 
        <tr>
            <td><code>GET_PIXEL</code></td>
            <td>pop int from <code>stack</code> as <code>index</code>, then push color at <code>pixel_buf[index]</code> on <code>stack</code></td>
        </tr>
        <tr>
            <td><code>BLIT</code></td>
            <td>yield <code>pixel_buf</code> from <code>VM</code> to library user</td>
        </tr>
        <tr>
            <td><code>RANDOM_INT</code></td>
            <td>pop int from <code>stack</code> as <code>rand_max</code>, then push random number in range <code>[0..max]</code>(exclusive) on <code>stack</code></td>
        </tr>
        <tr>
            <td rowspan=2><code>SPECIAL</code></td>
            <td><code>DUMP</code></td>
            <td>dumps <code>stack</code> to stdout</td>
        </tr>
        <tr>
            <td><code>SWAP</code></td>
            <td>swaps last two values on <code>stack</code></td>
        </tr>
    </tbody>
</table>

## Library documentation

1) Install [rustup](https://www.rust-lang.org/tools/install)
2) Clone repository
   ```
   git clone https://github.com/MabaKalox/animation-lang.git
   ```
3) Run command in project directory to generate and open docs in browser
   ```shell
   cargo doc --open
   ```

## Library usage example

### Basic

```rust
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
```

### More examples

Check `examples` directory, each example can be run by:

```shell
cargo run --example [example name] -- [args]
````

#### dummy_client

Renders led strip emulation on the screen and listen for binary progs
at http POST endpoint `0.0.0.0:8888/send_prog_base64`

#### compile

CLI tool to compile source code and optionally save or send

```
Usage: compile [OPTIONS] --in-file <IN_FILE>

Options:
  -i, --in-file <IN_FILE>      
  -o, --out-file <OUT_FILE>    
  -s, --send-addr <SEND_ADDR>  address to send base64 encoded program
  -h, --help                   Print help information
```

## License

This project is licensed under the MIT License - see the LICENSE.md file for details
