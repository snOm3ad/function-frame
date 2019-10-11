# Function Frames

A procedural macro to wrap any output inside a function within a text frame.

## The Problem

Do you often find yourself using `println!` for debugging purposes? If so, then you probably have experienced the pain of trying to debug the same program a week later. Suddenly, you can't remember the exact structure of the code and you find yourself seeing some output in your terminal but have no idea where it may be coming from. If this has happened to you then don't worry I built this crate for you (and myself).

Though, if I am being honest, the main use case of this crate is for learning. You see, I like to learn how to use libraries by writing my own compact examples inside functions and calling these functions to see if I get the behavior I was expecting. You can imagine that this approach take a toll with complex libraries as the number of examples grows pretty quickly. So I would often find myself writing code like this

```rust
fn example_one() {
    let title = "Example One";
    let width = 50;
    let header_sep = "=".repeat(width);
    let footer_sep = "=".repeat(2 * (width + 1) + title.len());
    println!("{} Example One {}", header_sep, header_sep);
    // some code here...
    println!("{}", footer_sep);
}
```
In order to get an output that looks like this,

```
================= Example One =================
// example output
===============================================
```

The problem is that I would have to write the same 6 lines of code for every example I created. Not only did this become tedious after a few examples, I also found that these lines ofuscated the code inside the example as they aren't doing any meaningful work. They are only there for aesthetics! 

## The Solution

Hence, I decided to create a procedural macro that takes care of mimicking the exact same behavior. So now instead of writing the above, with this crate I simply write

```rust
#[frame(title = "Example One", sep = "=", width = 50)]
fn example_one() {
    // some code here...
}
```

Which is a huge improvement, not only reduces the amount of code I have to write it also leaves the code inside the example intact! For now it only works by printing to `stdout` though in the future if the use case arises I will consider adding the functionality of writing to a file.
