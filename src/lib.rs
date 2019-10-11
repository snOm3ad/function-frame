extern crate proc_macro;

use proc_macro as pm;
use proc_macro2 as pm2;

use quote::ToTokens;
use syn::parse::{Parse, Parser};
use syn::punctuated::Punctuated;

struct HeaderConfig {
    str_opts: Vec<(String, String)>,
    num_opts: Vec<(String, usize)>,
    bin_opts: Vec<(String, bool)>,
}

//  The different types of options that we can expect from the user.
enum Opts {
    Num(usize),
    Str(String),
    Bin(bool),
}

impl HeaderConfig {
    fn new() -> Self {
        //  Pre-allocate memory for the number of options we are expecting.
        //  Though the user may give more than what we expect.
        HeaderConfig {
            num_opts: Vec::with_capacity(1),
            str_opts: Vec::with_capacity(2),
            bin_opts: Vec::with_capacity(1),
        }
    }
}

// TODO: refactor this so that it returns an error.
fn parse_macro_arguments(args: pm2::TokenStream) -> HeaderConfig {
    //  Use a `Punctuated` sequence of `syn::ExprAssign` which is basically
    //  things of the form:
    //      ```
    //          a = b
    //      ```
    //  which are separated by the `,` character! This parser will parse all
    //  the arguments until no further `syn::ExprAssign` are found.
    //
    //  In short, a `syn::ExprAssign` consists of the following elements:
    //      a)  attrs: Vec<syn::Attributes>
    //
    //          -   which is basically any macro arguments that are
    //              directly above the expression assignment.
    //
    //      b)  left: Box<Expr>
    //
    //          -   which most of the time is an identifier, but it
    //              sometimes be another `syn::ExprAssign` like for
    //              instance:
    //                  ` a = b = c `
    //
    //      c)  eq_token: syn::Token![=]
    //      d)  right: Box<Expr>
    //
    //          -   which most of the time is a literal, but it could
    //              also be something else as shown above.
    let expr_parser = Punctuated::<syn::ExprAssign, syn::Token![,]>::parse_terminated;

    //  Consume the argument tokenstream.
    let expressions = match Parser::parse2(expr_parser, args) {
        Ok(expressions) => {
            //  We cannot construct the headers if we do not have at least three
            //  arguments:
            //
            //      1. `title` 2. `sep` 3. `width`
            //
            //  So if the user only provides 2 or less we cannot construct the
            //  headers so we can safely panic.
            assert!(
                expressions.len() > 2,
                format!(
                    "expected at least 3 arguments received {}.",
                    expressions.len()
                )
            );
            //  Collect the expressions into a vector of `syn::ExprAssigns`
            expressions.into_iter().collect::<Vec<_>>()
        }
        Err(_) => {
            //  Happens whenever the arguments of the `attribute_macro` are
            //  not well constructed, e.g.
            //      ```
            //          #[add_headers(title: "", ...)]
            //      ```
            //  will not work because it expects and '=' sign, not a colon.
            //  Hence it's not a valid assignment.
            panic!("invalid list of expression arguments");
        }
    };

    //  The config object has three maps:
    //
    //      1. (String, usize) 2. (String, String) 3. (String, bool)
    //
    //  Each of them is separated because it makes it so much easier to work
    //  with the data when it's separated it. If you wanted to keep a single
    //  vector for all three value types (i.e. `usize`, `String` and `bool`).
    //
    //  Then you would constantly need to match against the multiple types
    //  that the element can be, even when you know for sure that an element
    //  with a key `K` is of type `T`.
    let mut config = HeaderConfig::new();

    //  Start looping through all the assignment expressions.
    //  NOTE that there is no limit as to how many of them the user is allowed
    //       pass, but we don't care about limiting this number because either
    //       way we are only going to use the one's we care about.
    for expr in expressions {
        //  Store the identifier always as a `String`.
        let lhs_expr = match *(expr.left) {
            syn::Expr::Path(p) => match p.path.get_ident() {
                Some(res) => res.to_string(),
                None => panic!("expected identifier, found `path`."),
            },
            _ => panic!("expected identifer, found something else."),
        };

        //  Match the `rhs` with literals only.
        let rhs_expr: Opts = match *(expr.right) {
            //  The top level match of the `syn::Expr` inside the box
            //  will produce a `syn::ExprLit` which has an element
            //  inside called `lit` which is of type `syn::Lit` which
            //  is an enum that allows us to match against several types
            //  of specific literals.
            //
            //  For now, we are interested in handling three kinds of
            //  literals:
            //
            //      a) string literals  b) binary literals  c) integer literals
            //
            //  If the user provides something that is not of these three
            //  types then we can safely panic.
            syn::Expr::Lit(expr) => match expr.lit {
                syn::Lit::Str(str_lit) => Opts::Str(str_lit.value()),
                syn::Lit::Bool(bin_lit) => Opts::Bin(bin_lit.value),
                syn::Lit::Int(num_lit) => {
                    Opts::Num(num_lit.base10_digits().parse::<usize>().unwrap())
                }
                _ => {
                    panic!("expected literal of type `bool`, `str` or `num`, found something else.")
                }
            },
            _ => panic!("expected literal, found something else."),
        };

        //  Place each literal and it's associated key in it's respective bucket
        //  inside the config object.
        match rhs_expr {
            Opts::Num(num_opt) => config.num_opts.push((lhs_expr, num_opt)),
            Opts::Bin(bin_opt) => config.bin_opts.push((lhs_expr, bin_opt)),
            Opts::Str(str_opt) => config.str_opts.push((lhs_expr, str_opt)),
        }
    }

    config
}

use std::{error, fmt};

#[derive(Clone, Debug)]
struct ArgNotFound<'a> {
    //  We create this custom error, to store the name of the missing argument.
    name: &'a str,
}

impl<'a> fmt::Display for ArgNotFound<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "expected argument with name '{}', found none.",
            self.name
        )
    }
}

impl<'a> error::Error for ArgNotFound<'a> {}

//  Helper function to help us locate a given argument name within a specified map
//  in the `HeaderConfig` object. NOTE that we have to take the map `Vec` as ref
//  and avoid moving values from it cause we might need them later.
fn find_argument<K, V>(map: &Vec<(K, V)>, arg_name: &'static str) -> Result<V, impl error::Error>
where
    K: PartialEq<str>,
    V: Clone,
{
    //  Function is simple, we are given the key we are interested in locating, and
    //  the vector it should be located on. Notice that the key is given in the form
    //  of `arg_name` which is of type `&str` so `K` needs to be comparable to a string.
    match map.iter().find(|(k, _)| k == arg_name) {
        //  If we find the key in the given vector, then we return its ssociated value.
        //  Since, we don't want to move the value from the Vector we are given we need
        //  the value type `V` to implement the `Clone` trait.
        Some((_, val)) => Ok(val.clone()),
        //  Else we return an error.
        None => Err(ArgNotFound { name: arg_name }),
    }
}

fn construct_guards(
    segment_title: String,
    sep: String,
    width: usize,
    sep_line: bool,
) -> (pm2::TokenStream, pm2::TokenStream) {
    //  The `sep_line` argument specifies whether the title should be printed in its
    //  own line or in a same line as the separators.
    if sep_line {
        //  If we want the segment title in it's own line we need to modify the width
        //  given by the user to account for that.
        let width = width + segment_title.len();
        let hsep = sep.repeat(width);
        let header = quote::quote! {
            //  The blank space in the `format!` macro tells rust to pad the segment
            //  title with whitespace, `width` number of times.
            println!("{}\n{}\n{}", #hsep, format!("{: ^1$}", #segment_title, #width), #hsep);
        };
        //  Constructing the `footer` is pretty much the same as with the header.
        let fsep = sep.repeat(width);
        let footer = quote::quote! {
            println!("{}", #fsep);
        };

        (header, footer)
    } else {
        //  Print the header and separators in the same line.
        let hsep = sep.repeat(width);
        let header = quote::quote! {
            println!("{} {} {}", #hsep, #segment_title, #hsep);
        };

        let fsep = sep.repeat(2 * (width + 1) + segment_title.len());
        let footer = quote::quote! {
            println!("{}", #fsep);
        };

        (header, footer)
    }
}

#[proc_macro_attribute]
pub fn frame(args: pm::TokenStream, item: pm::TokenStream) -> pm::TokenStream {
    //  Change the input to `proc_macro2::TokenStream` as `syn` and `quote` both
    //  work with this type of `TokenStream`, and it allows for compiler version
    //  independent code, and allows the code to exist outside the macro compila-
    //  tion level -- which means you can unit test it.
    let args = pm2::TokenStream::from(args.clone());
    //  Get the config object from the arguments passed by the user.
    let conf = parse_macro_arguments(args);

    let mut segment_title = match find_argument(&conf.str_opts, "title") {
        Ok(title) => title,
        Err(err) => panic!(format!("{}\nmake sure teh value is of type `str`.", err)),
    };

    //  For some reason the `"` character seems to be part of the `syn::Lit` type
    //  so even after we convert it to a string, we get something that is wrapped
    //  in quotes, which in this case is undersirable.
    segment_title.retain(|c| c != '\"');

    //  The separating character or string.
    let mut sep = match find_argument(&conf.str_opts, "sep") {
        Ok(sep) => sep,
        Err(err) => panic!(format!("{}\nmake sure the value is of type `str`.", err)),
    };

    sep.retain(|c| c != '\"');

    //  The number of times you want the separator character to be repeated.
    let width = match find_argument(&conf.num_opts, "width") {
        Ok(width) => width,
        Err(err) => panic!(format!("{}\nmake sure the value is of type `usize`.", err)),
    };

    //  NOTE this argument is really not that important in order to construct a header,
    //       so we can make it optional. notice there's no panic if the `find_argument`
    //       function returns an error.
    let sep_line = match find_argument(&conf.bin_opts, "sep_line") {
        Ok(sep_line) => sep_line,
        Err(_) => true,
    };

    //  Construct two `pm2::TokenStreams` using the `quote` crate.
    let (header, footer) = construct_guards(segment_title, sep, width, sep_line);

    //  Use `syn::Stmt::parse` function to parse the `pm2::TokenStreams` into `syn::Stmts`
    //  which makes it much more convenient to insert into the user's code.
    let macro_parser = syn::Stmt::parse;
    let header_macro_stmt = Parser::parse2(macro_parser, header).unwrap();
    let footer_macro_stmt = Parser::parse2(macro_parser, footer).unwrap();

    //  Finally we need to parse the input in order to determine someone is not calling this
    //  macro in a context where it doesn't make sense. Right now, this macro expects to be
    //  used only in functions.
    let input = pm2::TokenStream::from(item.clone());
    match Parser::parse2(syn::ItemFn::parse, input) {
        Ok(mut func) => {
            //  The `func.block.stmts` variable is of type `Vec<syn::Stmts>` so we can easily
            //  insert our header and footer guards without even having to touch the user's
            //  existing code.
            let n = func.block.stmts.len() + 1;
            func.block.stmts.insert(0, header_macro_stmt);
            func.block.stmts.insert(n, footer_macro_stmt);
            //  Finally now that everything is properly setup, we return the modified function.
            pm::TokenStream::from(func.to_token_stream())
        }
        Err(_) => panic!("macro can only be applied to `function` items."),
    }
}
