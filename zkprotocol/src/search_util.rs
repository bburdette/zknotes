use crate::search::{AndOr, SearchMod, TagSearch};
use nom::{
  branch::alt,
  bytes::complete::{tag, take_while1},
  character::complete::{char, multispace0},
  combinator::{map, value},
  multi::many0,
  sequence::{delimited, preceded},
  IResult, Parser,
};

// --- Parsers ---

fn search_mod(input: &str) -> IResult<&str, SearchMod> {
  alt((
    value(SearchMod::ExactMatch, tag("e")),
    value(SearchMod::ZkNoteId, tag("z")),
    value(SearchMod::Tag, tag("t")),
    value(SearchMod::Note, tag("n")),
    value(SearchMod::User, tag("u")),
    value(SearchMod::File, tag("f")),
    value(SearchMod::Before, tag("b")),
    value(SearchMod::After, tag("a")),
    value(SearchMod::Create, tag("c")),
    value(SearchMod::Mod, tag("m")),
    value(SearchMod::Server, tag("s")),
  ))
  .parse(input)
}

fn search_mods(input: &str) -> IResult<&str, Vec<SearchMod>> {
  many0(search_mod).parse(input)
}

fn search_term(input: &str) -> IResult<&str, String> {
  let quoted_content = many0(alt((
    // handle escaped single quote
    map(tag("\\'"), |_| "'".to_string()),
    // handle normal chars until single quote or backslash
    map(take_while1(|c| c != '\'' && c != '\\'), |s: &str| {
      s.to_string()
    }),
  )));
  delimited(char('\''), map(quoted_content, |v| v.concat()), char('\'')).parse(input)
}

// andor: & or |
fn andor(input: &str) -> IResult<&str, AndOr> {
  alt((value(AndOr::And, char('&')), value(AndOr::Or, char('|')))).parse(input)
}

// spaces: zero or more spaces
fn spaces(input: &str) -> IResult<&str, ()> {
  value((), multispace0).parse(input)
}

// oplistParser : Parser (List ( AndOr, TagSearch ))
fn oplist_parser(input: &str) -> IResult<&str, Vec<(AndOr, TagSearch)>> {
  many0((preceded(spaces, andor), preceded(spaces, tag_search_parser))).parse(input)
}

// singleTerm : Parser TagSearch
fn single_term(input: &str) -> IResult<&str, TagSearch> {
  alt((
    // mods + term
    map((search_mods, search_term), |(mods, term)| {
      TagSearch::SearchTerm { mods, term }
    }),
    // Not
    map(preceded((tag("!"), spaces), tag_search_parser), |term| {
      TagSearch::Not { ts: Box::new(term) }
    }),
    // Parenthesized
    delimited((tag("("), spaces), tag_search_parser, (spaces, tag(")"))),
  ))
  .parse(input)
}

pub fn tag_search_parser(input: &str) -> IResult<&str, TagSearch> {
  // single_term then zero or more (op, term)
  let (input, init_term) = single_term(input)?;
  let (input, op_terms) = oplist_parser(input)?;
  // Left fold the op_terms into Boolex
  let result = op_terms
    .into_iter()
    .fold(init_term, |acc, (op, term)| TagSearch::Boolex {
      ts1: Box::new(acc),
      ao: op,
      ts2: Box::new(term),
    });
  Ok((input, result))
}

// --- Usage Example ---

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_tag_search_parser() {
    // e.g. t'foo'&n'bar'
    let input = "t'foo'&n'bar'";
    let (_, ts) = tag_search_parser(input).unwrap();
    println!("{:?}", ts);
    // Should print a Boolex with ts1=TagSearchTerm([Tag],'foo'), ao=And, ts2=TagSearchTerm([Note],'bar')
  }

  #[test]
  fn test_not_and_parens() {
    let input = "!(t'foo'|n'bar')";
    let (_, ts) = tag_search_parser(input).unwrap();
    println!("{:?}", ts);
  }
}
