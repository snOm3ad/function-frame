use function_frame::add_headers;

#[add_headers(title = "Hello World - Example", sep = "-", width = 25)]
fn main() {
    println!("Hello, world!");
}
