use nom::{
  branch::alt,
  bytes::complete::{tag, take_while, take_while1},
  character::complete::{char, multispace0, one_of},
  combinator::{map, opt, recognize, value},
  multi::{many0, many1},
  sequence::{delimited, preceded, separated_pair, tuple},
  IResult, Parser,
};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchMod {
  ExactMatch,
  ZkNoteId,
  Tag,
  Note,
  User,
  File,
  Before,
  After,
  Create,
  Mod,
  Server,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AndOr {
  And,
  Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ST {
  pub mods: Vec<SearchMod>,
  pub term: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TagSearch {
  SearchTerm(ST),
  Not {
    ts: Box<TagSearch>,
  },
  Boolex {
    ts1: Box<TagSearch>,
    ao: AndOr,
    ts2: Box<TagSearch>,
  },
}

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

// Forward-declare tag_search_parser so we can use it recursively
fn tag_search_parser(input: &str) -> IResult<&str, TagSearch> {
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

// oplistParser : Parser (List ( AndOr, TagSearch ))
fn oplist_parser(input: &str) -> IResult<&str, Vec<(AndOr, TagSearch)>> {
  many0(tuple((
    preceded(spaces, andor),
    preceded(spaces, tag_search_parser),
  )))
  .parse(input)
}

// singleTerm : Parser TagSearch
fn single_term(input: &str) -> IResult<&str, TagSearch> {
  alt((
    // mods + term
    map(tuple((search_mods, search_term)), |(mods, term)| {
      TagSearch::SearchTerm(ST { mods, term })
    }),
    // Not
    map(
      preceded(tuple((tag("!"), spaces)), tag_search_parser),
      |term| TagSearch::Not { ts: Box::new(term) },
    ),
    // Parenthesized
    delimited(
      tuple((tag("("), spaces)),
      tag_search_parser,
      tuple((spaces, tag(")"))),
    ),
  ))
  .parse(input)
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
