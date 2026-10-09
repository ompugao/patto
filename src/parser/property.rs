use std::sync::Arc;

use pest::iterators::{Pair, Pairs};

use super::deadline::parse_deadline;
use super::{Deadline, Location, Property, Rule, Span, TaskStatus};
use crate::task::Duration;

pub(super) fn transform_property(
    pair: Pair<Rule>,
    input: &str,
    row: usize,
    offset: usize,
) -> Option<Property> {
    let location = Location {
        row,
        input: Arc::from(input),
        span: Span::from(pair.as_span()) + offset,
    };
    match pair.as_rule() {
        Rule::expr_anchor => Some(short_anchor(pair, location)),
        Rule::expr_property => named_property(pair, location),
        Rule::expr_task => Some(shorthand_task(pair, location)),
        other => panic!("Unhandled token: {:?}", other),
    }
}

fn short_anchor(pair: Pair<Rule>, location: Location) -> Property {
    Property::Anchor {
        name: pair.into_inner().next().unwrap().as_str().to_string(),
        location,
    }
}

fn named_property(pair: Pair<Rule>, location: Location) -> Option<Property> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str();
    match name {
        "anchor" => long_anchor(inner, location),
        "task" => Some(task_property(inner, location)),
        _ => {
            log::warn!("Unknown property: {}", name);
            None
        }
    }
}

fn long_anchor(mut args: Pairs<Rule>, location: Location) -> Option<Property> {
    let Some(name) = args.next() else {
        log::warn!("Anchor property missing name");
        return None;
    };
    Some(Property::Anchor {
        name: name.as_str().to_string(),
        location,
    })
}

fn task_property(args: Pairs<Rule>, location: Location) -> Property {
    let mut fields = TaskFields::default();
    for arg in args {
        match arg.as_rule() {
            Rule::property_keyword_pair => {
                let mut key_value = arg.into_inner();
                let key = key_value.next().unwrap().as_str();
                let value = key_value.next().unwrap().as_str();
                fields.set(key, value);
            }
            Rule::property_positional_arg => {
                log::warn!(
                    "Unexpected positional arg in task property: {}",
                    arg.as_str()
                );
            }
            other => log::warn!("Unexpected rule in task property: {:?}", other),
        }
    }
    fields.into_property(location)
}

#[derive(Default)]
struct TaskFields {
    status: TaskStatus,
    status_is_canonical: bool,
    due: Option<Deadline>,
    scheduled: Option<Deadline>,
    completed_at: Option<Deadline>,
    started_at: Option<Deadline>,
    time_spent: Option<Duration>,
}

impl TaskFields {
    fn set(&mut self, key: &str, value: &str) {
        match key {
            "status" => match TaskStatus::from_keyword(value) {
                Some(status) => {
                    self.status = status;
                    self.status_is_canonical = true;
                }
                None => {
                    log::warn!("Unknown task status: '{}', interpreted as 'todo'", value);
                    self.status = TaskStatus::Todo;
                }
            },
            "due" => self.due = Some(parse_deadline(value)),
            "scheduled" => self.scheduled = Some(parse_deadline(value)),
            "completed_at" => self.completed_at = Some(parse_deadline(value)),
            "started_at" => self.started_at = Some(parse_deadline(value)),
            "time_spent" => self.time_spent = value.parse().ok(),
            _ => log::warn!("Unknown task property key: {}", key),
        }
    }

    fn into_property(self, location: Location) -> Property {
        Property::Task {
            status: self.status,
            status_is_canonical: self.status_is_canonical,
            due: self
                .due
                .unwrap_or_else(|| Deadline::Uninterpretable(String::new())),
            scheduled: self.scheduled,
            completed_at: self.completed_at,
            started_at: self.started_at,
            time_spent: self.time_spent,
            location,
        }
    }
}

fn shorthand_task(pair: Pair<Rule>, location: Location) -> Property {
    let mut inner = pair.into_inner();
    let status = match inner.next().unwrap().as_rule() {
        Rule::symbol_task_done => TaskStatus::Done,
        Rule::symbol_task_doing => TaskStatus::Doing,
        Rule::symbol_task_todo => TaskStatus::Todo,
        other => unreachable!("expr_task starts with a task symbol, got {:?}", other),
    };
    Property::Task {
        status,
        status_is_canonical: true,
        due: parse_deadline(inner.as_str()),
        scheduled: None,
        completed_at: None,
        started_at: None,
        time_spent: None,
        location,
    }
}
