use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

use git2::Repository;
use iced::advanced::mouse::Cursor;
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::{Button, Interaction, ScrollDelta};
use iced::widget::canvas::{Program, Geometry, Frame, Path, Style, Text, Stroke, Event};
use iced::widget::{text, Column, Row, Canvas};
use iced::{Alignment, Element, Sandbox, Settings, Length, Rectangle, Theme, Color, mouse, Renderer, Point, Vector};

use crate::backend::{CommitNode, get_commit_depth, get_commit_height};

struct SharedState {
    commits: HashMap<String, CommitNode>,
    selected_commit: Option<String>,
}

pub struct GitUI {
    repository: Repository,
    state: Rc<RefCell<SharedState>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshTree,
    SelectCommit(String),
    UnselectCommit,
}

impl GitUI {
    pub fn start() {
        Self::run(Settings {
            antialiasing: true,
            ..Default::default()
        }).unwrap()
    }
}

impl Sandbox for GitUI {
    type Message = Message;

    fn new() -> Self {
        let repository = match Repository::open(".") {
            Ok(repository) => repository,
            Err(e) => panic!("Error opening repository: {}", e),
        };

        let state = SharedState { commits: HashMap::new(), selected_commit: None };

        let mut ui = Self { repository, state: Rc::new(RefCell::new(state)) };
        ui.update(Message::RefreshTree);

        ui
    }

    fn title(&self) -> String {
        String::from("GitUI")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::RefreshTree => {
                let state = &mut *self.state.borrow_mut();

                let references = self.repository.references().unwrap();
                for reference in references {
                    let as_commit = reference.unwrap().peel_to_commit();

                    match as_commit.ok() {
                        Some(commit) => { CommitNode::create(commit, &mut state.commits); },
                        None => (),
                    }
                }
            },
            Message::SelectCommit(commit) => {
                self.state.borrow_mut().selected_commit = Some(commit.clone());
            },
            Message::UnselectCommit => {
                self.state.borrow_mut().selected_commit = None;
            },
        }
    }

    fn view(&self) -> Element<Message> {
        Column::with_children({
            let mut children: Vec<Element<Message>> = Vec::new();

            children.push(
                Row::with_children({
                    vec![
                        text("Commits").size(30).into()
                    ]
                })
                .align_items(Alignment::Center)
                .spacing(10)
                .into());

            children.push(Row::with_children({
                let mut children = Vec::new();

                children.push(Row::with_children({
                    let selected_commit = &self.state.borrow().selected_commit;
                    if let Some(selected) = selected_commit {
                        vec![
                            text(format!("Current: {}", &selected)).size(20).into(),
                        ]
                    } else {
                        Vec::new()
                    }
                })
                .height(30)
                .align_items(Alignment::Start)
                .spacing(10)
                .into());

                children
            }).into());

            children.push(Canvas::new(TreeRenderer { state: Rc::clone(&self.state) })
                .width(Length::Fill)
                .height(Length::Fill)
                .into());

            children
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }
}

struct TreeRenderer {
    state: Rc<RefCell<SharedState>>,
}

const NODE_RADIUS: f32 = 50.0;

fn get_commit_node_location(commit: &CommitNode, commits: &HashMap<String, CommitNode>) -> Point {
    let x = get_commit_depth(commit, commits) as f32 * NODE_RADIUS * 2.5;
    let y = get_commit_height(commit, commits) as f32 * NODE_RADIUS * 1.5;
    Point::new(x, y)
}

fn adjust_position_for_view(position: Point, bounds: &Rectangle, state: &TreeState) -> Point {
    let x = state.zoom * (position.x + state.offset.x) + bounds.width / 2.0;
    let y = state.zoom * (position.y + state.offset.y) + bounds.height / 2.0;
    Point::new(x, y)
}

struct TreeState {
    mouse_location: Point,
    dragging: bool,
    offset_start: Vector,
    dragging_start: Point,
    offset: Vector,
    zoom: f32,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            mouse_location: Default::default(),
            dragging: Default::default(),
            offset_start: Default::default(),
            dragging_start: Default::default(),
            offset: Default::default(),
            zoom: 1.0,
        }
    }
}

impl Program<Message> for TreeRenderer {
    type State = TreeState;

    fn update(&self, state: &mut Self::State, event: Event, bounds: Rectangle, _cursor: Cursor) -> (Status, Option<Message>) {
        let commits = &self.state.borrow().commits;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                if button == Button::Left {
                    if state.mouse_location.y > 0.0 {
                        for commit in commits.values() {
                            let location = get_commit_node_location(commit, commits);
                            let location = adjust_position_for_view(location, &bounds, state);

                            if state.mouse_location.distance(location) < NODE_RADIUS * state.zoom {
                                return (Status::Captured, Some(Message::SelectCommit(commit.id.clone())))
                            }
                        }

                        state.dragging = true;
                        state.dragging_start = state.mouse_location;
                        state.offset_start = state.offset;

                        (Status::Captured, Some(Message::UnselectCommit))
                    } else {
                        (Status::Captured, None)
                    }
                } else {
                    (Status::Ignored, None)
                }
            },
            Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                if button == Button::Left {
                    if state.dragging {
                        state.dragging = false;
                    }

                    (Status::Captured, None)
                } else {
                    (Status::Ignored, None)
                }
            },
            Event::Mouse(mouse::Event::CursorMoved { position: location }) => {
                state.mouse_location = Point::new(location.x - bounds.x, location.y - bounds.y);

                if state.dragging {
                    state.offset = state.offset_start + (state.mouse_location - state.dragging_start) * (1.0 / state.zoom);
                }

                (Status::Captured, None)
            },
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !state.dragging {
                    if let ScrollDelta::Lines { x: _, y } = delta {
                        // want to get the distance the origin changes from the scroll, then
                        // subtract it
                        //
                        // get mouse location and subtract center to get distance between them
                        // multiply that by zoom

                        //let x = state.mouse_location.x - bounds.width / 2.0;

                        state.zoom += y * 0.15 * state.zoom;
                        state.zoom = state.zoom.clamp(0.25, 4.0);

                        //let xReal = state.offset.x;//(state.mouse_location.x - bounds.width / 2.0) * previous;

                        // Calculate the XY as if the element is in its original, non-scaled size:
                        //let xOrg = xReal / previous;

                        // Calculate the scaled XY
                        //let xNew = xOrg * state.zoom;  // PS: scale here is the new scale.

                        // Retrieve the XY difference to be used as the change in offset.
                        //let xDiff = xReal - xNew;

                        //state.offset.x += xDiff;
                        //let target = (state.mouse_location.x - (bounds.width / 2.0 + state.offset.x)) / previous;

                        //state.offset.x = -target * state.zoom;


                        //println!("{}", ((state.offset.x * previous + state.mouse_location.x) - (state.offset.x * state.zoom + bounds.width / 2.0)) * (state.zoom - previous) * state.zoom);
                        //state.offset.x -= (state.mouse_location.x - bounds.width / 2.0) / previous / 3.0 * (state.zoom - previous);
                        //state.offset.x -= ((state.offset.x * previous + state.mouse_location.x) - (state.offset.x * state.zoom + bounds.width / 2.0)) * (state.zoom - previous) * state.zoom;

                        //state.offset.x -= ((state.offset.x + state.mouse_location.x) - (state.offset.x + bounds.width / 2.0)) * (state.zoom - previous) * state.zoom;
                        //state.offset.y -= ((state.offset.y + state.mouse_location.y) - (state.offset.y + bounds.height / 2.0)) * (state.zoom - previous) * state.zoom;
                    }
                }
                (Status::Captured, None)
            },
            _ => (Status::Ignored, None),
        }
    }

    fn mouse_interaction(&self, state: &Self::State, bounds: Rectangle, _cursor: Cursor) -> Interaction {
        let commits = &self.state.borrow().commits;

        if state.dragging {
            Interaction::Grabbing
        } else {
            for commit in commits.values() {
                let location = get_commit_node_location(commit, commits);
                let location = adjust_position_for_view(location, &bounds, state);

                if state.mouse_location.distance(location) < NODE_RADIUS * state.zoom {
                    return Interaction::Pointer
                }
            }

            if state.mouse_location.y > 0.0 {
                Interaction::Pointer
            } else {
                Interaction::Idle
            }
        }
    }

    fn draw(&self, state: &TreeState, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let commits = &self.state.borrow().commits;

        let mut frame = Frame::new(renderer, bounds.size());

        for commit in self.state.borrow().commits.values() {
            let location = get_commit_node_location(commit, commits);
            let location = adjust_position_for_view(location, &bounds, state);

            let node = Path::circle(location, NODE_RADIUS * state.zoom);
            frame.fill(&node, Color::from_rgb(0.35, 0.35, 0.35));

            let text = Text {
                content: commit.id[..6].to_string(),
                position: location,
                size: 15.0 * state.zoom,
                color: Color::from_rgb(0.8, 0.8, 0.8),
                horizontal_alignment: Horizontal::Center,
                vertical_alignment: Vertical::Center,
                ..Default::default()
            };

            frame.fill_text(text);

            if commit.parents.len() > 0 {
                let parent_location = get_commit_node_location(commits.get(commit.parents.get(0).unwrap()).unwrap(), commits);
                let parent_location = adjust_position_for_view(parent_location, &bounds, state);
                let path = Path::line(Point::new(location.x - NODE_RADIUS * state.zoom, location.y), Point::new(parent_location.x + NODE_RADIUS * state.zoom, parent_location.y));
                frame.stroke(&path, Stroke {
                    width: 2.0,
                    style: Style::Solid(Color::BLACK),
                    ..Default::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}
