use std::sync::Arc;

use pest::iterators::Pair;

use super::deadline::parse_deadline;
use super::{Deadline, Location, Property, Rule, Span, TaskStatus};

pub(super) fn transform_property(
    pair: Pair<Rule>,
    input: &str,
    row: usize,
    offset: usize,
) -> Option<Property> {
    let span = Span::from(pair.as_span()) + offset;
    let location = Location {
        row,
        input: Arc::from(input),
        span: span.clone(),
    };

    match pair.as_rule() {
        Rule::expr_anchor => {
            let anchor = Property::Anchor {
                name: pair.into_inner().next().unwrap().as_str().to_string(),
                location,
            };
            Some(anchor)
        }
        Rule::expr_property => {
            let mut inner = pair.into_inner();
            let property_name = inner.next().unwrap().as_str();

            match property_name {
                "anchor" => {
                    // Long form anchor: {@anchor name}
                    // Expect one positional argument (the anchor name)
                    let anchor_name = inner.next().map(|p| p.as_str().to_string());
                    if let Some(name) = anchor_name {
                        Some(Property::Anchor { name, location })
                    } else {
                        log::warn!("Anchor property missing name");
                        None
                    }
                }
                "task" => {
                    // Task property: {@task status=todo due=2024-12-31 scheduled=2024-12-30 completed_at=2024-12-31}
                    let mut status = TaskStatus::Todo;
                    let mut status_is_canonical = false;
                    let mut due = Deadline::Uninterpretable("".to_string());
                    let mut scheduled: Option<Deadline> = None;
                    let mut completed_at: Option<Deadline> = None;
                    let mut started_at: Option<Deadline> = None;
                    let mut time_spent: Option<crate::task::Duration> = None;
                    let mut current_key = "";

                    for kv in inner {
                        match kv.as_rule() {
                            Rule::property_keyword_pair => {
                                // Parse key=value pair
                                let mut pair_inner = kv.into_inner();
                                let key = pair_inner.next().unwrap().as_str();
                                let value = pair_inner.next().unwrap().as_str();

                                if key == "status" {
                                    status = match value {
                                        "todo" => {
                                            status_is_canonical = true;
                                            TaskStatus::Todo
                                        }
                                        "doing" | "inprogress" | "wip" => {
                                            status_is_canonical = true;
                                            TaskStatus::Doing
                                        }
                                        "paused" => {
                                            status_is_canonical = true;
                                            TaskStatus::Paused
                                        }
                                        "done" => {
                                            status_is_canonical = true;
                                            TaskStatus::Done
                                        }
                                        _ => {
                                            log::warn!(
                                                "Unknown task status: '{}', interpreted as 'todo'",
                                                value
                                            );
                                            TaskStatus::Todo
                                        }
                                    };
                                } else if key == "due" {
                                    due = parse_deadline(value);
                                } else if key == "scheduled" {
                                    scheduled = Some(parse_deadline(value));
                                } else if key == "completed_at" {
                                    completed_at = Some(parse_deadline(value));
                                } else if key == "started_at" {
                                    started_at = Some(parse_deadline(value));
                                } else if key == "time_spent" {
                                    time_spent = value.parse().ok();
                                } else {
                                    log::warn!("Unknown task property key: {}", key);
                                }
                            }
                            Rule::property_keyword_arg => {
                                current_key = kv.as_str();
                            }
                            Rule::property_keyword_value => {
                                let value = kv.as_str();
                                if current_key == "status" {
                                    status = match value {
                                        "todo" => {
                                            status_is_canonical = true;
                                            TaskStatus::Todo
                                        }
                                        "doing" => {
                                            status_is_canonical = true;
                                            TaskStatus::Doing
                                        }
                                        "paused" => {
                                            status_is_canonical = true;
                                            TaskStatus::Paused
                                        }
                                        "done" => {
                                            status_is_canonical = true;
                                            TaskStatus::Done
                                        }
                                        _ => {
                                            log::warn!(
                                                "Unknown task status: '{}', interpreted as 'todo'",
                                                value
                                            );
                                            TaskStatus::Todo
                                        }
                                    };
                                } else if current_key == "due" {
                                    due = parse_deadline(value);
                                } else if current_key == "scheduled" {
                                    scheduled = Some(parse_deadline(value));
                                } else if current_key == "completed_at" {
                                    completed_at = Some(parse_deadline(value));
                                } else if current_key == "started_at" {
                                    started_at = Some(parse_deadline(value));
                                } else if current_key == "time_spent" {
                                    time_spent = value.parse().ok();
                                } else {
                                    log::warn!("Unknown task property value: {}", value);
                                }
                            }
                            Rule::property_positional_arg => {
                                log::warn!(
                                    "Unexpected positional arg in task property: {}",
                                    kv.as_str()
                                );
                            }
                            _ => {
                                log::warn!("Unexpected rule in task property: {:?}", kv.as_rule());
                            }
                        }
                    }
                    Some(Property::Task {
                        status,
                        status_is_canonical,
                        due,
                        scheduled,
                        completed_at,
                        started_at,
                        time_spent,
                        location,
                    })
                }
                _ => {
                    log::warn!("Unknown property: {}", property_name);
                    None
                }
            }
        }
        Rule::expr_task => {
            let mut inner = pair.into_inner();
            let symbol = inner.by_ref().next().unwrap();
            let status = match symbol.as_rule() {
                Rule::symbol_task_done => TaskStatus::Done,
                Rule::symbol_task_doing => TaskStatus::Doing,
                Rule::symbol_task_todo => TaskStatus::Todo,
                _ => unreachable!(),
            };
            let due_str = inner.as_str();
            let due = parse_deadline(due_str);
            Some(Property::Task {
                status,
                status_is_canonical: true,
                due,
                scheduled: None,
                completed_at: None,
                started_at: None,
                time_spent: None,
                location,
            })
        }
        _ => {
            panic!("Unhandled token: {:?}", pair.as_rule());
        }
    }
}
