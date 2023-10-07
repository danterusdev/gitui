use git2::{Repository, IndexAddOption};
use iced::widget::{button, text, Column, Row};
use iced::{Alignment, Element, Sandbox, Settings};

pub struct GitUI {
    repository: Repository,
    remotes: Vec<String>,
    changes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshRemotes,
    RefreshChanged,
    StageChange(String),
}

impl GitUI {
    pub fn start() {
        Self::run(Settings::default()).unwrap()
    }
}

impl Sandbox for GitUI {
    type Message = Message;

    fn new() -> Self {
        let repository = match Repository::open(".") {
            Ok(repository) => repository,
            Err(e) => panic!("Error opening repository: {}", e),
        };

        let mut ui = Self { repository, remotes: Vec::new(), changes: Vec::new() };
        ui.update(Message::RefreshRemotes);
        ui.update(Message::RefreshChanged);

        ui
    }

    fn title(&self) -> String {
        String::from("GitUI")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::RefreshRemotes => {
                let remotes = self.repository.remotes().unwrap();
                self.remotes.clear();
                for remote in remotes.iter() {
                    self.remotes.push(String::from(remote.unwrap()));
                }
            },
            Message::RefreshChanged => {
                let changes = self.repository.diff_index_to_workdir(None, None).unwrap();
                self.changes.clear();
                for change in changes.deltas() {
                    self.changes.push(String::from(change.new_file().path().unwrap().to_str().unwrap()));
                }
            },
            Message::StageChange(file) => {
                let mut index = self.repository.index().unwrap();
                index.add_all([file].iter(), IndexAddOption::DEFAULT, None).unwrap();
                index.write().unwrap();
                self.update(Message::RefreshChanged);
            },
        }
    }

    fn view(&self) -> Element<Message> {
        Column::with_children({
            let mut children = Vec::new();

            children.push(Column::with_children({
                let mut children = Vec::new();

                children.push(Row::with_children({
                    vec![
                        text("Remotes").size(30).into()
                    ]
                })
                .align_items(Alignment::Center)
                .spacing(10)
                .into());

                for remote in &self.remotes {
                    children.push(text(remote).size(20).into());
                }

                children
            }).into());

            children.push(Column::with_children({
                let mut children = Vec::new();

                children.push(Row::with_children({
                    vec![
                        text("Changed").size(30).into(),
                    ]
                })
                .align_items(Alignment::Center)
                .spacing(10)
                .into());

                for change in &self.changes {
                    children.push(Row::with_children({
                        vec![
                            button(text("Stage").size(12.5)).on_press(Message::StageChange(change.clone())).into(),
                            text(change).size(20).into(),
                        ]
                    })
                    .align_items(Alignment::Center)
                    .spacing(10)
                    .into());
                }

                children
            }).into());

            children
        })
        .padding(20)
        .spacing(10)
        .align_items(Alignment::Start)
        .into()
    }
}
