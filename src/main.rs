#![feature(iterator_try_collect)]

use rand::prelude::*;

use ratatui::{prelude::*, widgets::{Block, Paragraph}};
use layout::Position;
use style::Color;

use std::collections::HashSet;

use crossterm::event::*;

#[derive(PartialEq, Eq, Hash)]
struct Wall {
    position: Vec<usize>,
    axis: usize,
}

impl Wall {
    /// Get the list of walls neighbouring a cell at position.
    pub fn from_cell(shape: &[usize], position: &[usize]) -> Vec<Wall> {
        let mut walls = Vec::new();
        for axis in 0..shape.len() {
            for sign in [false, true] {
                let mut position = position.to_vec();
                if sign {
                    if position[axis] != 0 {
                        position[axis] -= 1;
                    } else {
                        position[axis] = shape[axis] - 1;
                    }
                }
                walls.push(Wall { position, axis });
            }
        }
        walls
    }

    /// Get the two cells neighbouring the wall.
    pub fn get_neighbour_cells(&self, shape: &[usize]) -> [Vec<usize>; 2] {
        let position1 = self.position.clone();

        let mut position2 = self.position.clone();
        if position2[self.axis] != shape[self.axis] - 1 {
            position2[self.axis] += 1;
        } else {
            position2[self.axis] = 0;
        }

        [position1, position2]
    }
}

struct Maze {
    dimensions: Vec<usize>,

    start: Vec<usize>,
    end: Vec<usize>,

    position: Vec<usize>,
    axes: [usize; 2],

    walls: Vec<bool>,
}

impl Maze {
    pub fn new(dimensions: Vec<usize>) -> Maze {
        let start = vec![0; dimensions.len()];
        let end = vec![0; dimensions.len()];
        let position = vec![0; dimensions.len()];

        let axes = [0, 1];

        let wall_count = dimensions.iter().product::<usize>() * dimensions.len();
        let walls = vec![true; wall_count];

        Maze {
            dimensions,
            start,
            end,
            position,
            axes,
            walls,
        }
    }

    pub fn compute_wall_index(&self, wall: &Wall) -> usize {
        let mut index = 0;
        let mut stride = 1;

        for (limit, value) in std::iter::zip(self.dimensions.iter(), wall.position.iter()) {
            index += stride * *value;
            stride *= *limit;
        }

        index += stride * wall.axis;
        index
    }

    pub fn reset_walls(&mut self) {
        self.walls.fill(true);
    }

    pub fn get_wall(&self, wall: &Wall) -> bool {
        let index = self.compute_wall_index(wall);
        self.walls[index]
    }

    pub fn set_wall(&mut self, wall: &Wall, value: bool) {
        let index = self.compute_wall_index(wall);
        self.walls[index] = value;
    }

    pub fn generate<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        for (limit, value) in std::iter::zip(self.dimensions.iter(), self.start.iter_mut()) {
            *value = rng.random_range(0..*limit);
        }

        for (limit, value) in std::iter::zip(self.dimensions.iter(), self.end.iter_mut()) {
            *value = rng.random_range(0..*limit);
        }

        // Yep. This waste a lot of memory, but apparently who cares?
        let mut visited = HashSet::<Vec<usize>>::from_iter([self.start.clone()]);
        let mut walls = Wall::from_cell(&self.dimensions, &self.start);

        self.reset_walls();
        while !walls.is_empty() {
            let wall = walls.swap_remove(rng.random_range(0..walls.len()));

            let mut okay = false;
            for cell in wall.get_neighbour_cells(&self.dimensions) {
                if !visited.contains(&cell) {
                    for wall in Wall::from_cell(&self.dimensions, &cell) {
                        if self.get_wall(&wall) {
                            walls.push(wall);
                        }
                    }
                    visited.insert(cell);
                    okay = true;
                }
            }

            if okay {
                self.set_wall(&wall, false);
            }
        }
    }

    pub fn start(&mut self) {
        self.position.copy_from_slice(&self.start);
    }

    pub fn walk(&mut self, view_axis: usize, sign: bool) {
        if sign {
            if self.get_wall(&Wall { position: self.position.clone(), axis: self.axes[view_axis] }) {
                return;
            }

            if self.position[self.axes[view_axis]] != self.dimensions[self.axes[view_axis]] - 1 {
                self.position[self.axes[view_axis]] += 1;
            } else {
                self.position[self.axes[view_axis]] = 0;
            }
        } else {
            let old_value = self.position[self.axes[view_axis]];

            if self.position[self.axes[view_axis]] != 0 {
                self.position[self.axes[view_axis]] -= 1;
            } else {
                self.position[self.axes[view_axis]] = self.dimensions[self.axes[view_axis]] - 1;
            }

            if self.get_wall(&Wall { position: self.position.clone(), axis: self.axes[view_axis] }) {
                self.position[self.axes[view_axis]] = old_value;
            }
        }
    }

    pub fn set_view_axis(&mut self, view_axis: usize, axis : usize) {
        if view_axis < 2 && axis < self.dimensions.len() {
            self.axes[view_axis] = axis;
        }
    }
}

impl Widget for &Maze {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        let height = area.height;
        let width = area.width / 2;

        for y in 0..height {
            for x in 0..width {
                let wy = y as isize - (height / 2) as isize;
                let wx = x as isize - (width / 2) as isize;

                enum RenderCell {
                    Wall,
                    Empty,
                    Start,
                    End,
                    Current,
                }

                match match (wy.rem_euclid(2), wx.rem_euclid(2)) {
                    (1, 1) => RenderCell::Wall,
                    (ry, rx)  => {
                        let mut position = self.position.clone();
                        position[self.axes[0]] = (position[self.axes[0]] as isize + wy.div_euclid(2)).rem_euclid(self.dimensions[self.axes[0]] as isize) as usize;
                        position[self.axes[1]] = (position[self.axes[1]] as isize + wx.div_euclid(2)).rem_euclid(self.dimensions[self.axes[1]] as isize) as usize;
                        match (ry, rx) {
                            (0, 0) => {
                                if position == self.start {
                                    RenderCell::Start
                                } else if position == self.end {
                                    RenderCell::End
                                } else if position == self.position {
                                    RenderCell::Current
                                } else {
                                    RenderCell::Empty
                                }
                            },
                            (1, 0) => if self.get_wall(&Wall { position, axis: self.axes[0] }) { RenderCell::Wall } else { RenderCell::Empty },
                            (0, 1) => if self.get_wall(&Wall { position, axis: self.axes[1] }) { RenderCell::Wall } else { RenderCell::Empty },
                            _ => unreachable!(),
                        }
                    },
                } {
                    RenderCell::Wall => {
                        buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█');
                        buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█');
                    },
                    RenderCell::Empty => {
                        buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char(' ');
                        buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char(' ');
                    },
                    RenderCell::Start => {
                        buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█').set_fg(Color::Green);
                        buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█').set_fg(Color::Green);
                    },
                    RenderCell::End => {
                        buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█').set_fg(Color::Red);
                        buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█').set_fg(Color::Red);
                    },
                    RenderCell::Current => {
                        buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█').set_fg(Color::Yellow);
                        buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█').set_fg(Color::Yellow);
                    },
                }
            }
        }
    }
}

enum Application {
    Menu {
        dimension: String,
    },
    Main {
        maze: Maze,
        view_axis : Option<usize>,
    },
}

fn parse_dimension(s: &str) -> Option<Vec<usize>> {
    s
        .split(',')
        .map(|s| s.trim())
        .map(|s| s.parse())
        .try_collect()
        .ok()
}

impl Application {
    pub fn new() -> Application {
        Self::Menu { dimension: String::new() }
    }

    pub fn run(&mut self) {
        let mut terminal = ratatui::init();
        loop {
            terminal.draw(|frame| self.render(frame)).unwrap();
            if !self.update() {
                break
            }
        }
        ratatui::restore();
    }

    pub fn render(&self, frame: &mut Frame) {
        match self {
            Application::Menu { dimension } => {
                let text = if dimension.is_empty() {
                    Text::from(" Enter dimension of maze to be generated here: (e.g. 50, 40, 30) ").style(Style::new().dark_gray())
                } else {
                    if parse_dimension(dimension).is_some() {
                        Text::from(format!(" Dimension: {dimension} ")).style(Style::new().green())
                    } else {
                        Text::from(format!(" Dimension: {dimension} ")).style(Style::new().red())
                    }
                };

                let desired_width = (text.width() + 2) as u16;
                let desired_height = 3;

                let mut input_area = frame.area();

                if input_area.width > desired_width {
                    input_area.x += (input_area.width - desired_width) / 2;
                    input_area.width = desired_width;
                }

                if input_area.height > desired_height {
                    input_area.y += (input_area.height - desired_height) / 2;
                    input_area.height = desired_height;
                }

                let input_widget = Paragraph::new(text).block(Block::bordered());
                frame.render_widget(input_widget, input_area);
            },
            Application::Main { maze, view_axis } => {
                let mut info = Text::default();

                {
                    let mut line = Line::default();
                    line.push_span("Current Axes (Vertical, Horizontal): ");

                    let mut span = Span::raw(format!("{}", maze.axes[0]));
                    if *view_axis == Some(0) { span = span.style(Style::new().red()); }
                    line.push_span(span);

                    line.push_span(" ");

                    let mut span = Span::raw(format!("{}", maze.axes[1]));
                    if *view_axis == Some(1) { span = span.style(Style::new().red()); }
                    line.push_span(span);

                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("Dimensions: ");
                    for (i, dimension) in maze.dimensions.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("Position: ");
                    for (i, dimension) in maze.position.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("Start: ");
                    for (i, dimension) in maze.start.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("End: ");
                    for (i, dimension) in maze.end.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                let mut help = Text::default();

                match view_axis {
                    Some(_) => {
                        let mut line = Line::default();
                        line.push_span(format!("0-{}: Select replacement axis", maze.dimensions.len()-1));
                        help.push_line(line);

                        let mut line = Line::default();
                        line.push_span("Esc: Cancel selection of replacment axis");
                        help.push_line(line);

                    },
                    None => {
                        let mut line = Line::default();
                        line.push_span("0-1: Select which axes to modify");
                        help.push_line(line);

                        let mut line = Line::default();
                        line.push_span("Esc: exit");
                        help.push_line(line);
                    },
                }

                {
                    let mut line = Line::default();
                    line.push_span("Arrow Keys: Move");
                    help.push_line(line);
                }

                let [info_area, help_area, maze_area] = Layout::vertical([
                    Constraint::Length((info.lines.len()+2).try_into().unwrap()),
                    Constraint::Length((help.lines.len()+2).try_into().unwrap()),
                    Constraint::Min(0),
                ]).areas(frame.area());

                let info_block = Block::bordered().title("Info");
                let help_block = Block::bordered().title("Help");

                frame.render_widget(&info_block, info_area);
                frame.render_widget(&info, info_block.inner(info_area));

                frame.render_widget(&help_block, help_area);
                frame.render_widget(&help, help_block.inner(help_area));

                frame.render_widget(maze, maze_area);
            },
        }
    }

    pub fn update(&mut self) -> bool {
        let event = read().unwrap();
        match event {
            Event::Key(key_event) => match key_event {
                KeyEvent { code : KeyCode::Char('c'), modifiers : KeyModifiers::CONTROL, .. } => return false,
                KeyEvent { code : KeyCode::Char('q'), .. } => return false,
                _ => {},
            },
            _ => {},
        };

        match self {
            Application::Menu { dimension } => {
                match event {
                    Event::Key(key_event) => match key_event {
                        KeyEvent { code : KeyCode::Char(c), .. } => { dimension.push(c); },
                        KeyEvent { code : KeyCode::Esc, .. } => { dimension.clear(); },
                        KeyEvent { code : KeyCode::Backspace, .. } => { dimension.pop(); },
                        KeyEvent { code : KeyCode::Enter, .. } => {
                            if let Some(dimension) = parse_dimension(dimension) {
                                let mut maze = Maze::new(dimension);
                                maze.generate(&mut rand::rng());
                                maze.start();
                                *self = Application::Main { maze, view_axis : None }
                            }
                        },
                        _ => {},
                    },
                    _ => {},
                }
            },
            Application::Main { maze, view_axis } => {
                match event {
                    Event::Key(key_event) => match key_event {
                        KeyEvent { code : KeyCode::Up, .. } => maze.walk(0, false),
                        KeyEvent { code : KeyCode::Down, .. } => maze.walk(0, true),
                        KeyEvent { code : KeyCode::Left, .. } => maze.walk(1, false),
                        KeyEvent { code : KeyCode::Right, .. } => maze.walk(1, true),

                        KeyEvent { code : KeyCode::Esc, .. } => {
                            match view_axis {
                                Some(_) => *view_axis = None,
                                None => *self = Application::new(),
                            }
                        },

                        KeyEvent { code : KeyCode::Char(c), .. } if c >= '0' && c <= '9' => {
                            let d = c as usize - '0' as usize;
                            match view_axis.take() {
                                Some(view_axis) => maze.set_view_axis(view_axis, d),
                                None => *view_axis = Some(d),
                            }

                        },

                        _ => {},
                    },
                    _ => {},
                }
            },
        }

        true
    }
}

fn main() {
    Application::new().run()
}
