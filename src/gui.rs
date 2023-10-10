use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

use git2::{Repository, Oid};
use iced::advanced::mouse::Cursor;
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::{Button, Interaction, ScrollDelta};
use iced::widget::canvas::{Program, Geometry, Frame, Path, Style, Text, Stroke, Event};
use iced::widget::{text, Column, Row, Canvas, button};
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
    SwitchToCommit(String),
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
                    let reference = reference.unwrap();
                    let reference_name = reference.name().unwrap().to_string().clone();
                    assert!(reference_name.contains('/'));
                    let reference_name = reference_name[reference_name.rfind('/').unwrap() + 1..].to_string();
                    let as_commit = reference.peel_to_commit();

                    match as_commit.ok() {
                        Some(commit) => { CommitNode::create(commit, &mut state.commits, Some(reference_name)); },
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
            Message::SwitchToCommit(commit) => {
                let commits = &self.state.borrow().commits;
                let commit_node = commits.get(&commit).unwrap();
                let object = if let Some(reference) = &commit_node.reference {
                    self.repository.find_object(self.repository.refname_to_id(&format!("refs/heads/{}", reference)).unwrap(), None).unwrap()
                } else {
                    self.repository.find_object(Oid::from_str(&commit).unwrap(), None).unwrap()
                };
                self.repository.checkout_tree(&object, None).unwrap();
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
                            text(format!("ID: {}", &selected)).size(20).into(),
                            button("Checkout").on_press(Message::SwitchToCommit(selected.clone())).into()
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

fn adjust_position_for_view(position: &Point, bounds: &Rectangle, state: &TreeState) -> Point {
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
    initialized: bool,
    node_locations: HashMap<String, Point>,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            mouse_location: Default::default(),
            dragging: Default::default(),
            offset_start: Default::default(),
            dragging_start: Default::default(),
            offset: Default::default(),
            initialized: Default::default(),
            node_locations: Default::default(),
            zoom: 1.0,
        }
    }
}

impl Program<Message> for TreeRenderer {
    type State = TreeState;

    fn update(&self, state: &mut Self::State, event: Event, bounds: Rectangle, _cursor: Cursor) -> (Status, Option<Message>) {
        if !state.initialized {
            let commits = &self.state.borrow().commits;
            for (id, commit) in commits.iter() {
                let location = get_commit_node_location(commit, commits);
                state.node_locations.insert(id.clone(), location);
            }
            state.initialized = true;
        }

        let commits = &self.state.borrow().commits;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                if button == Button::Left {
                    if state.mouse_location.y > 0.0 {
                        for id in commits.keys() {
                            let location = state.node_locations.get(id).unwrap();
                            let location = adjust_position_for_view(location, &bounds, state);

                            if state.mouse_location.distance(location) < NODE_RADIUS * state.zoom {
                                return (Status::Captured, Some(Message::SelectCommit(id.clone())))
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
                        // Mouse location in terms of graph coordinates
                        let mouse_location_x = (state.mouse_location.x - bounds.width / 2.0) / state.zoom - state.offset.x;
                        let mouse_location_y = (state.mouse_location.y - bounds.height / 2.0) / state.zoom - state.offset.y;

                        let previous_pos = Point::new(mouse_location_x, mouse_location_y);
                        // Previous position of mouse in screen coordinates
                        let previous_pos = adjust_position_for_view(&previous_pos, &bounds, state);

                        state.zoom += y * 0.15 * state.zoom;
                        state.zoom = state.zoom.clamp(0.001, 4.0);

                        let new_pos = Point::new(mouse_location_x, mouse_location_y);
                        // Current position of mouse in screen coordinates
                        let new_pos = adjust_position_for_view(&new_pos, &bounds, state);

                        // Mouse distance moved in graph coordinates
                        let moved_x = (new_pos.x - previous_pos.x) / state.zoom;
                        state.offset.x -= moved_x;
                        let moved_y = (new_pos.y - previous_pos.y) / state.zoom;
                        state.offset.y -= moved_y;
                    }
                }
                (Status::Captured, None)
            },
            _ => (Status::Ignored, None),
        }
    }

    fn mouse_interaction(&self, state: &Self::State, bounds: Rectangle, _cursor: Cursor) -> Interaction {
        if !state.initialized {
            return Default::default();
        }

        let commits = &self.state.borrow().commits;

        if state.dragging {
            Interaction::Grabbing
        } else {
            for id in commits.keys() {
                let location = state.node_locations.get(id).unwrap();
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
        if !state.initialized {
            return vec![];
        }

        let commits = &self.state.borrow().commits;

        let mut frame = Frame::new(renderer, bounds.size());

        for (id, commit) in commits.iter() {
            let location = state.node_locations.get(id).unwrap();
            let location = adjust_position_for_view(&location, &bounds, state);

            if location.x > bounds.x + bounds.width || location.x < bounds.x ||
                location.y > bounds.y + bounds.height || location.y < bounds.y {
                continue
            }

            let node = Path::circle(location, NODE_RADIUS * state.zoom);
            frame.fill(&node, Color::from_rgb(0.35, 0.35, 0.35));

            let text = Text {
                content: id[..6].to_string(),
                position: location,
                size: 15.0 * state.zoom,
                color: Color::from_rgb(0.8, 0.8, 0.8),
                horizontal_alignment: Horizontal::Center,
                vertical_alignment: Vertical::Center,
                ..Default::default()
            };

            frame.fill_text(text);

            if let Some(reference) = &commit.reference {
                let text = Text {
                    content: reference.to_string(),
                    position: Point::new(location.x, location.y - NODE_RADIUS * 1.2 * state.zoom),
                    size: 15.0 * state.zoom,
                    color: Color::from_rgb(0.2, 0.2, 0.2),
                    horizontal_alignment: Horizontal::Center,
                    vertical_alignment: Vertical::Center,
                    ..Default::default()
                };

                frame.fill_text(text);
            }

            for parent in &commit.parents {
                let parent_location = state.node_locations.get(parent).unwrap();
                let parent_location = adjust_position_for_view(&parent_location, &bounds, state);
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
