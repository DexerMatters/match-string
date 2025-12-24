use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Token, parse_macro_input};

struct MatchesInput {
    reference: Expr,
    _arrow: Token![=>],
    pattern: PatternExpr,
}

struct PatternExpr {
    kind: PatternKind,
}

enum PatternKind {
    Lit(syn::Lit),
    Ident(Ident),
    Tuple(Vec<PatternExpr>),
    Or(Vec<PatternExpr>),
    Many(Box<PatternExpr>),
    Some(Box<PatternExpr>),
    Sep(Box<PatternExpr>, Box<PatternExpr>),
    Sep1(Box<PatternExpr>, Box<PatternExpr>),
    To(Ident, Box<PatternExpr>),
}

impl Parse for MatchesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let reference: Expr = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let pattern = input.parse::<PatternExpr>()?;
        Ok(MatchesInput {
            reference,
            _arrow,
            pattern,
        })
    }
}

impl Parse for PatternExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        parse_seq_expr(input)
    }
}

fn parse_seq_expr(input: ParseStream) -> syn::Result<PatternExpr> {
    let mut terms = Vec::new();
    loop {
        terms.push(parse_or_expr(input)?);
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else {
            break;
        }
    }
    if terms.len() == 1 {
        Ok(terms.into_iter().next().unwrap())
    } else {
        Ok(PatternExpr {
            kind: PatternKind::Tuple(terms),
        })
    }
}

fn parse_or_expr(input: ParseStream) -> syn::Result<PatternExpr> {
    let mut terms = Vec::new();
    loop {
        terms.push(parse_and_expr(input)?);
        if input.peek(Token![/]) {
            input.parse::<Token![/]>()?;
        } else {
            break;
        }
    }
    if terms.len() == 1 {
        Ok(terms.into_iter().next().unwrap())
    } else {
        Ok(PatternExpr {
            kind: PatternKind::Or(terms),
        })
    }
}

fn parse_and_expr(input: ParseStream) -> syn::Result<PatternExpr> {
    let expr = parse_term(input)?;
    // support bracketed separator syntax: `elem[sep]+` => Sep(elem, sep)
    if input.peek(syn::token::Bracket) {
        let content;
        syn::bracketed!(content in input);
        let sep = parse_or_expr(&content)?;
        if input.peek(Token![+]) {
            input.parse::<Token![+]>()?;
            return Ok(PatternExpr {
                kind: PatternKind::Sep(Box::new(expr), Box::new(sep)),
            });
        } else if input.peek(Token![*]) {
            input.parse::<Token![*]>()?;
            return Ok(PatternExpr {
                kind: PatternKind::Sep1(Box::new(expr), Box::new(sep)),
            });
        } else {
            return Err(syn::Error::new(
                input.span(),
                "expected `+` or `*` after bracketed separator",
            ));
        }
    }

    if input.peek(Token![+]) {
        input.parse::<Token![+]>()?;
        Ok(PatternExpr {
            kind: PatternKind::Many(Box::new(expr)),
        })
    } else if input.peek(Token![*]) {
        input.parse::<Token![*]>()?;
        Ok(PatternExpr {
            kind: PatternKind::Some(Box::new(expr)),
        })
    } else {
        Ok(expr)
    }
}

fn parse_term(input: ParseStream) -> syn::Result<PatternExpr> {
    if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        // parentheses act as grouping/sequence; parse inner sequence expression
        let inner = parse_seq_expr(&content)?;
        return Ok(inner);
    }

    if input.peek(Ident) && input.peek2(Token![@]) {
        let ident: Ident = input.parse()?;
        input.parse::<Token![@]>()?;
        let expr = parse_or_expr(input)?;
        return Ok(PatternExpr {
            kind: PatternKind::To(ident, Box::new(expr)),
        });
    }

    if input.peek(syn::Lit) {
        let lit: syn::Lit = input.parse()?;
        return Ok(PatternExpr {
            kind: PatternKind::Lit(lit),
        });
    }

    if input.peek(Ident) {
        let ident: Ident = input.parse()?;
        return Ok(PatternExpr {
            kind: PatternKind::Ident(ident),
        });
    }

    Err(syn::Error::new(
        input.span(),
        "expected literal, identifier, grouped expression, or to",
    ))
}

fn build_pattern_tokens(pattern: &PatternExpr) -> proc_macro2::TokenStream {
    match &pattern.kind {
        PatternKind::Lit(lit) => quote! { #lit },
        PatternKind::Ident(ident) => quote! { #ident },
        PatternKind::Or(exprs) => {
            if exprs.is_empty() {
                panic!("empty or");
            } else if exprs.len() == 1 {
                build_pattern_tokens(&exprs[0])
            } else {
                let mut tokens = build_pattern_tokens(&exprs[0]);
                for expr in &exprs[1..] {
                    let inner = build_pattern_tokens(expr);
                    tokens = quote! { Or(#tokens, #inner) };
                }
                tokens
            }
        }
        PatternKind::Tuple(exprs) => {
            let inner = exprs.iter().map(build_pattern_tokens);
            quote! { (#(#inner),*) }
        }
        PatternKind::Many(expr) => {
            let inner = build_pattern_tokens(expr);
            quote! { RangeToInclusive { end: #inner } }
        }
        PatternKind::Some(expr) => {
            let inner = build_pattern_tokens(expr);
            quote! { RangeTo { end: #inner } }
        }
        PatternKind::Sep(elem, sep) => {
            let e = build_pattern_tokens(elem);
            let s = build_pattern_tokens(sep);
            quote! { Sep(#s, #e) }
        }
        PatternKind::Sep1(elem, sep) => {
            let e = build_pattern_tokens(elem);
            let s = build_pattern_tokens(sep);
            quote! { Sep1(#s, #e) }
        }
        PatternKind::To(ident, expr) => {
            let inner = build_pattern_tokens(expr);
            quote! { To(#inner, &#ident) }
        }
    }
}

#[proc_macro]
pub fn matches(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as MatchesInput);

    let pattern_tokens = build_pattern_tokens(&input.pattern);

    let reference = input.reference;

    let output = quote!({
        let __pattern = #pattern_tokens;
        crate::__matches(&__pattern, & #reference)
    });

    output.into()
}
