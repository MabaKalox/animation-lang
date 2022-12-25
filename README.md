# Animation-lang library

animation-lang library is a compiler and vm couple for primitive self-made programming language,
called `animation-language`. Its goal is to provide method for creating animations on addressable led
strips on the fly, without requiring re-flash of microcontroller which drives led strip.

<!-- TODO - content list -->

## Animation-lang syntax

Syntax of this programming language is mostly `c`-like, 
each expression and some statements should be followed by `;`

### Use literals (decimal or hexadecimals)
Just numbers like `12` or `0x2F`, minimal value: 0, maxim: 2^32-1

### Do simple mathematics

basic: `+` `-` `*` `/` `%`

logical: `==` `!=` `>` `>=` `<` `<=`

bitwise: `<<` `>>` `&` `|` `^` `!`

### Define constants

Assign value of expression to named constants
```
some_var1 = 10 + 20;
some_var2 = 10 << 8;
```
You can not create constant with used name, until previous one with such name leaves scope.

### Statements

#### forever `loop`

Code in such loop will be executed forever, no break method provided.
```
loop {
   ...
}
```

#### `for` loop

Loop with limited number of iterations, should be followed by `;`
```
for([counter_variable]=[number of iterations]) {
...
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

### Comments

You can write comments in code.

#### Single line comments
everything after `//` on the same line will be ignored by compiler
```
// comment on separate line
i = 10; // comment on same line
```

#### Multi line comments
everything after `/*` and before `*/` will be ignored by compiler
```
/* one line */
b = 0; /* same line,
but multiline!
*/
```

### Example programs

You can find examples of programs to understand syntax better in `example_progs` directory.

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

TODO

### Detailed

In `examples` directory you can find examples, which can be run by:

```shell
cargo run --example [example name] -- [args, e.g -h]
```

#### dummy_client
Renders led strip emulation on the screen and listen for compiled progs
at http POST endpoint `0.0.0.0:8888/send_prog_base64`

#### compile
CLI tool to compile source code and optionally:
* save to file
* send compiled program in base64 encoding in http POST request on provided endpoint

## License

This project is licensed under the MIT License - see the LICENSE.md file for details
