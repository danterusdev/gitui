use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

use git2::Repository;
use iced::advanced::mouse::Cursor;
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::{Button, Interaction};
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
    Point::new(position.x + bounds.width / 2.0 + state.offset.x, position.y + bounds.height / 2.0 + state.offset.y)
}

#[derive(Default)]
struct TreeState {
    mouse_location: Point,
    dragging: bool,
    offset_start: Vector,
    dragging_start: Point,
    offset: Vector,
}

impl Program<Message> for TreeRenderer {
    type State = TreeState;

    fn update(
            &self,
            state: &mut Self::State,
            event: Event,
            bounds: Rectangle,
            _cursor: Cursor,
        ) -> (Status, Option<Message>) {
        let commits = &self.state.borrow().commits;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                if button == Button::Left {
                    if state.mouse_location.y > 0.0 {
                        for commit in self.state.borrow().commits.values() {
                            let location = get_commit_node_location(commit, commits);
                            let location = adjust_position_for_view(location, &bounds, state);

                            if state.mouse_location.distance(location) < NODE_RADIUS {
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
                    state.offset = state.offset_start + (state.mouse_location - state.dragging_start);
                }

                (Status::Captured, None)
            }
            _ => (Status::Ignored, None),
        }
    }

    fn mouse_interaction(
            &self,
            state: &Self::State,
            _bounds: Rectangle,
            _cursor: Cursor,
        ) -> Interaction {
        if state.dragging {
            Interaction::Grabbing
        } else {
            Interaction::Grab
        }
    }

    fn draw(&self, state: &TreeState, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let commits = &self.state.borrow().commits;

        let mut frame = Frame::new(renderer, bounds.size());

        for commit in self.state.borrow().commits.values() {
            let location = get_commit_node_location(commit, commits);
            let location = adjust_position_for_view(location, &bounds, state);

            let node = Path::circle(location, NODE_RADIUS);
            frame.fill(&node, Color::from_rgb(0.35, 0.35, 0.35));

            let text = Text {
                content: commit.id[..6].to_string(),
                position: location,
                size: 15.0,
                color: Color::from_rgb(0.8, 0.8, 0.8),
                horizontal_alignment: Horizontal::Center,
                vertical_alignment: Vertical::Center,
                ..Default::default()
            };

            frame.fill_text(text);

            if commit.parents.len() > 0 {
                let parent_location = get_commit_node_location(commits.get(commit.parents.get(0).unwrap()).unwrap(), commits);
                let parent_location = adjust_position_for_view(parent_location, &bounds, state);
                let path = Path::line(Point::new(location.x - NODE_RADIUS, location.y), Point::new(parent_location.x + NODE_RADIUS, parent_location.y));
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
