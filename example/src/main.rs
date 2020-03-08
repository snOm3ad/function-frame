use function_frame::frame;

#[frame(title = "Simple Example", sep = "-", width = 25)]
fn void_func() {
    println!("I am simple.");
}

#[frame(title = "Returning", sep = "-", width = 25)]
fn nonvoid_func(val: bool) -> Option<isize> {
    println!("Returning!");
    if val {
        Some(-1)
    } else {
        None
    }
}

fn main() {
    void_func();
    let _: Option<isize> = nonvoid_func(true);
}
