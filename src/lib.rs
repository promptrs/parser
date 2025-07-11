#[allow(warnings)]
mod bindings;

use bindings::exports::promptrs::parser::response::{Delims, Guest, Response, ToolCall};
use serde::Deserialize;
use winnow::combinator::{alt, empty, opt, repeat, seq};
use winnow::error::ParserError;
use winnow::token::{rest, take_until};
use winnow::{Parser, Result};

struct Component;

impl Guest for Component {
	fn parse(response: String, delims: Option<Delims>) -> Response {
		parse(&mut response.as_str(), delims).unwrap_or(Response {
			reasoning: None,
			content: response,
			tool_calls: vec![],
		})
	}
}

fn parse(input: &mut &str, delims: Option<Delims>) -> Result<Response> {
	let Some(Delims {
		reasoning,
		tool_call: delims,
	}) = delims
	else {
		return Ok(Response {
			reasoning: None,
			content: input.to_string(),
			tool_calls: vec![],
		});
	};
	let Some(rdelims) = reasoning else {
		return seq!(Response {
			reasoning: empty.value(None),
			content: alt((take_until(0.., delims.0.as_str()), rest)).map(|s: &str| s.into()),
			tool_calls: repeat(0.., between(&delims)).map(parse_args)
		})
		.parse_next(input);
	};

	seq!(Response {
		reasoning: opt(between(&rdelims)).map(|s: Option<&str>| s.map(|s| s.into())),
		content: alt((take_until(0.., delims.0.as_str()), rest)).map(|s: &str| s.into()),
		tool_calls: repeat(0.., between(&delims)).map(parse_args)
	})
	.parse_next(input)
}

fn between<'s, E: ParserError<&'s str>>(
	(start, end): &(String, String),
) -> impl Parser<&'s str, &'s str, E> {
	|input: &mut &'s str| {
		let (mut start, mut end) = (start.as_str(), end.as_str());
		_ = take_until(0.., start).parse_next(input)?;
		_ = start.parse_next(input)?;
		let between = take_until(0.., end).parse_next(input)?;
		_ = end.parse_next(input)?;
		Ok(between)
	}
}

fn parse_args(list: Vec<&str>) -> Vec<ToolCall> {
	list.into_iter()
		.map(|tc| {
			match serde_json::from_str(tc).unwrap_or(ToolCallSegment::One(ToolCallDef {
				name: "".into(),
				arguments: "".into(),
			})) {
				ToolCallSegment::One(def) => vec![def],
				ToolCallSegment::Many(defs) => defs,
			}
		})
		.flatten()
		.map(|ToolCallDef { name, arguments }| ToolCall { name, arguments })
		.collect()
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ToolCallSegment {
	One(ToolCallDef),
	Many(Vec<ToolCallDef>),
}

#[derive(Deserialize)]
struct ToolCallDef {
	name: String,
	arguments: String,
}

bindings::export!(Component with_types_in bindings);
