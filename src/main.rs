#[allow(dead_code)]
mod util;

use std::time::Duration;
use std::sync::mpsc;
use std::thread;

use crossterm::{input, AlternateScreen, InputEvent, KeyEvent};
use tui::Terminal;
use tui::backend::CrosstermBackend;
use tui::style::{Style, Color};
use tui::layout::{Constraint, Layout};
use tui::widgets::{Widget, Block, Borders, SelectableList, canvas::Canvas};
use rand::prelude::*;

enum Event<I> {
    Input(I),
    Tick,
}

use ItemType::*;

#[derive(Copy, Clone, PartialEq)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Copy, Clone)]
enum ItemType {
    Apple,
    Mushroom,
    Hedgehog,
    Boulder,
}

#[derive(Copy, Clone)]
struct Segment {
    x: f64,
    y: f64,
    direction: Direction,
}

struct Item {
    item_type: ItemType,
    x: f64,
    y: f64,
}

struct App {
    segments: Vec<Segment>,
    items: Vec<Item>,
    playing: bool,
    canvas_x_length: f64,
    canvas_y_length: f64,
}

// Macro to generate random coordinates on the canvas
macro_rules! random_coordinates {
    ($rng:expr, $x_length:expr, $y_length:expr) => {
        ($rng.gen_range(2, $x_length as usize) as f64, $rng.gen_range(1, $y_length as usize) as f64)
    }
}

// Macro to increment a coordinate by 1 and overflow at the edges
macro_rules! increment_coordinate {
    ($value:expr, $overflow_after:expr) => {
        $value = if $value >= $overflow_after {
            $overflow_after - $value - 1.0
        } else {
            $value + 1.0
        };
    }
}

// Macro to decrement a coordinate by 1 and overflow at the edges
macro_rules! decrement_coordinate {
    ($value:expr, $overflow_to:expr) => {
        $value = if $value <= 0.0 {
            $overflow_to + $value
        } else {
            $value - 1.0
        };
    }
}

impl Default for Segment {
    fn default() -> Self {
        Segment {
            x: 1.0,
            y: 0.0,
            direction: Direction::Right,
        }
    }
}

impl App {
    fn new() -> App {
        App {
            segments: vec![
                Segment::default(),
                Segment { x: 2.0, ..Default::default() },
                Segment { x: 3.0, ..Default::default() },
                Segment { x: 4.0, ..Default::default() },
                Segment { x: 5.0, ..Default::default() },
                Segment { x: 6.0, ..Default::default() },
                Segment { x: 7.0, ..Default::default() },
                Segment { x: 8.0, ..Default::default() },
                Segment { x: 9.0, ..Default::default() },
                Segment { x: 10.0, ..Default::default() },
                Segment { x: 11.0, ..Default::default() },
                Segment { x: 12.0, ..Default::default() },
                Segment { x: 13.0, ..Default::default() },
            ],
            items: Vec::new(),
            playing: false,
            canvas_x_length: 10.0,
            canvas_y_length: 10.0,
        }
    }

    // Function to set the direction the snake should head in
    fn set_heading(&mut self, direction: Direction) {
        // Find the index of the head segment
        let head_index = self.segments.len() - 1;
        // Change just the direction of the head segment
        self.segments[head_index].direction = direction;
    }

    // Function to generate items on the canvas
    fn generate_item(&mut self) {
        let mut generate_destructive_item = false;
        let mut rng = rand::thread_rng();
        loop {
            // Generate random coordinates for the new item
            let (mut x, mut y) = random_coordinates!(rng, self.canvas_x_length - 1.0, self.canvas_y_length - 1.0);
            // Loop to see if the generated coordinates are free
            loop {
                // Check if the coordinates are occupied by the snake
                for segment in &self.segments {
                    if segment.x == x && segment.y == y {
                        // The coordinates aren't free; try again
                        let new_coordinates = random_coordinates!(rng, self.canvas_x_length - 1.0, self.canvas_y_length - 1.0);
                        x = new_coordinates.0;
                        y = new_coordinates.1;
                        // Skip to the next iteration
                        continue;
                    }
                }
                // Check if the coordinates are occupied by other items
                for item in &self.items {
                    if item.x == x && item.y == y {
                        // The coordinates aren't free; try again
                        let new_coordinates = random_coordinates!(rng, self.canvas_x_length - 1.0, self.canvas_y_length - 1.0);
                        x = new_coordinates.0;
                        y = new_coordinates.1;
                        // Skip to the next iteration
                        continue;
                    }
                }
                // The coordinates are free; break out of the loop
                break;
            }
            // Add the new item to the app instance
            let items = if generate_destructive_item {
                vec![Hedgehog, Boulder]
            } else {
                vec![Apple, Mushroom]
            };
            self.items.push(Item {
                item_type: *items.choose(&mut rng).unwrap(),
                x: x as f64,
                y: y as f64,
            });
            // A 20% chance to add a destructive item
            if rng.gen_bool(0.2) {
                generate_destructive_item = true;
                continue;
            }
            break;
        }
    }

    // Function that's called every tick
    fn update(&mut self) {
        // Move all the snake's segments 1 space in their respective directions
        for i in 0..self.segments.len() {
            match self.segments[i].direction {
                Direction::Up => increment_coordinate!(self.segments[i].y, self.canvas_y_length),
                Direction::Right => increment_coordinate!(self.segments[i].x, self.canvas_x_length),
                Direction::Down => decrement_coordinate!(self.segments[i].y, self.canvas_y_length),
                Direction::Left => decrement_coordinate!(self.segments[i].x, self.canvas_x_length),
            }
            // Update the segment's direction if it needs to change in the next tick
            if i < self.segments.len() - 1 && self.segments[i].direction != self.segments[i + 1].direction {
                self.segments[i].direction = self.segments[i + 1].direction;
            }
        }

        // Get the coordinates of the head
        let (head_x, head_y) = (self.segments[self.segments.len() - 1].x, self.segments[self.segments.len() - 1].y);

        // Check if the head's in the same space as any item
        for i in 0..self.items.len() {
            if head_x == self.items[i].x && head_y == self.items[i].y {
                // Remove the item from the app instance
                match self.items.remove(i).item_type {
                    Apple | Mushroom => {
                        // Create a new tail segment
                        let (mut x, mut y, direction) = (self.segments[0].x, self.segments[0].y, self.segments[0].direction);
                        match direction {
                            Direction::Up => decrement_coordinate!(y, self.canvas_y_length),
                            Direction::Right => decrement_coordinate!(x, self.canvas_x_length),
                            Direction::Down => increment_coordinate!(y, self.canvas_y_length),
                            Direction::Left => increment_coordinate!(x, self.canvas_x_length),
                        }
                        // Add the tail segment to the app instance
                        self.segments.insert(0, Segment { x, y, direction });
                    }
                    Hedgehog => {
                        // Remove the tail segment
                        // TODO end game if the snake's length is 0
                        if self.segments.len() > 1 {
                            self.segments.remove(0);
                        }
                    }
                    Boulder => {
                        // Move the head to either its right or its left
                        // Select the direction
                        let mut rng = rand::thread_rng();
                        let head_index = self.segments.len() - 1;
                        let (head_x, head_y) = (self.segments[head_index].x, self.segments[head_index].y);
                        let head_direction = self.segments[head_index].direction;
                        self.segments[head_index].direction = *if head_direction == Direction::Up || head_direction == Direction::Down {
                            [Direction::Right, Direction::Left]
                        } else {
                            [Direction::Up, Direction::Down]
                        }.choose(&mut rng).unwrap();
                        // Move the head in the selected direction
                        match self.segments[head_index].direction {
                            Direction::Up => increment_coordinate!(self.segments[head_index].y, self.canvas_y_length),
                            Direction::Right => increment_coordinate!(self.segments[head_index].x, self.canvas_x_length),
                            Direction::Down => decrement_coordinate!(self.segments[head_index].y, self.canvas_y_length),
                            Direction::Left => decrement_coordinate!(self.segments[head_index].x, self.canvas_x_length),
                        }
                        // Bring the head back one space
                        match self.segments[head_index - 1].direction {
                            Direction::Up => decrement_coordinate!(self.segments[head_index].y, self.canvas_y_length),
                            Direction::Right => decrement_coordinate!(self.segments[head_index].x, self.canvas_x_length),
                            Direction::Down => increment_coordinate!(self.segments[head_index].y, self.canvas_y_length),
                            Direction::Left => increment_coordinate!(self.segments[head_index].x, self.canvas_x_length),
                        }
                        self.segments[head_index - 1].direction = self.segments[head_index].direction;
                        // Bring the boulder back
                        self.items.push(Item {
                            item_type: Boulder,
                            x: head_x,
                            y: head_y,
                        });
                    }
                }
                return;
            }
        }

        // Check if the head's in the same space as any of the snake's other segments
        let initial_body_length = self.segments.len() - 1;
        for i in 0..initial_body_length {
            if head_x == self.segments[i].x && head_y == self.segments[i].y {
                // Remove the cut segments from the app instance
                while self.segments.len() > initial_body_length - i {
                    self.segments.remove(0);
                }
                return;
            }
        }
    }
}

fn main() -> Result<(), failure::Error> {
    let screen = AlternateScreen::to_alternate(true)?;
    let backend = CrosstermBackend::with_alternate_screen(screen)?;
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Create app instance
    let mut app = App::new();
    // Variable to keep track of whether a game is currently in progress
    let mut game_in_progress = false;
    // Variable to keep track of when new items should be generated
    let mut need_items_in = 0;
    // Variable to keep track of the selected option in the menu
    let mut selected_option = 0;

    // Setup input handling
    let (tx, rx) = mpsc::channel();
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let input = input();
            let reader = input.read_sync();
            for event in reader {
                match event {
                    InputEvent::Keyboard(key) => {
                        if let Err(_) = tx.send(Event::Input(key.clone())) {
                            return;
                        }
                        if key == KeyEvent::Char('q') {
                            return;
                        }
                    }
                    _ => {}
                }
            }
        });
    }
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let tx = tx.clone();
            loop {
                tx.send(Event::Tick).unwrap();
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    terminal.clear()?;

    // Main app loop
    loop {
        if app.playing {
            terminal.draw(|mut f| {
                let size = f.size();
                let rects = Layout::default()
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(size);
                app.canvas_x_length = size.width as f64;
                app.canvas_y_length = size.height as f64;

                if need_items_in == 0 {
                    app.generate_item();
                    need_items_in = 15;
                }

                need_items_in -= 1;

                Canvas::default()
                    .block(Block::default().borders(Borders::NONE))
                    .paint(|ctx| {
                        let (mut tongue_x, mut tongue_y) = (app.segments[app.segments.len() - 1].x, app.segments[app.segments.len() - 1].y);
                        let (head, tongue) = match app.segments[app.segments.len() - 1].direction {
                            Direction::Up => {
                                increment_coordinate!(tongue_y, app.canvas_y_length);
                                ("▲", "↑")
                            }
                            Direction::Right => {
                                increment_coordinate!(tongue_x, app.canvas_x_length);
                                ("", "→")
                            }
                            Direction::Down => {
                                decrement_coordinate!(tongue_y, app.canvas_y_length);
                                ("▼", "↓")
                            }
                            Direction::Left => {
                                decrement_coordinate!(tongue_x, app.canvas_x_length);
                                ("", "←")
                            }
                        };
                        for (i, segment) in app.segments.iter().rev().enumerate() {
                            ctx.print(segment.x, segment.y, if i == 0 { head } else if i % 2 == 0 { "█" } else { "▓" }, Color::Indexed(191));
                            if i == 0 && (((app.segments[app.segments.len() - 1].direction == Direction::Right || app.segments[app.segments.len() - 1].direction == Direction::Left) && tongue_x % 4.0 == 0.0) || ((app.segments[app.segments.len() - 1].direction == Direction::Up || app.segments[app.segments.len() - 1].direction == Direction::Down) && tongue_y % 4.0 == 0.0)) {
                                ctx.print(tongue_x, tongue_y, tongue, Color::Indexed(196));
                            }
                        }

                        for item in &app.items {
                            match item.item_type {
                                Apple => ctx.print(item.x, item.y, "🍎", Color::Indexed(160)),
                                Mushroom => ctx.print(item.x, item.y, "🍄", Color::Indexed(166)),
                                Hedgehog => ctx.print(item.x, item.y, "🦔", Color::Indexed(216)),
                                Boulder => ctx.print(item.x, item.y, "🟤", Color::Indexed(95)),
                            }
                        }
                    })
                .x_bounds([0.0, size.width as f64])
                    .y_bounds([0.0, size.height as f64])
                    .render(&mut f, rects[0]);
            })?;

            let current_heading = app.segments[app.segments.len() - 1].direction;
            match rx.recv()? {
                Event::Input(input) => match input {
                    KeyEvent::Char('q') => {
                        // Quit the program
                        break;
                    }
                    KeyEvent::Up => if current_heading != Direction::Down {
                        // Change the snake's head's direction to up
                        app.set_heading(Direction::Up)
                    }
                    KeyEvent::Right => if current_heading != Direction::Left {
                        // Change the snake's head's direction to right
                        app.set_heading(Direction::Right)
                    }
                    KeyEvent::Down => if current_heading != Direction::Up {
                        // Change the snake's head's direction to down
                        app.set_heading(Direction::Down)
                    }
                    KeyEvent::Left => if current_heading != Direction::Right {
                        // Change the snake's head's direction to left
                        app.set_heading(Direction::Left)
                    }
                    _ => {}
                }
                Event::Tick => app.update()
            }
        } else {
            let mut menu_items = vec!["New Game       (n)", "Leaderboards   (l)", "Settings       (s)", "Help           (h)", "Quit           (q)"];
            terminal.draw(|mut f| {
                // Draw menu
                if game_in_progress {
                    menu_items.insert(0, "Resume Game    (r)");
                }
                let chunks = Layout::default()
                    .margin(5)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(f.size());
                SelectableList::default()
                    .block(Block::default().borders(Borders::ALL).title("Snake"))
                    .items(&menu_items)
                    .select(Some(selected_option))
                    .style(Style::default().fg(Color::Indexed(204)))
                    .highlight_style(Style::default().fg(Color::Indexed(207)))
                    .highlight_symbol("→")
                    .render(&mut f, chunks[0]);
            })?;

            match rx.recv()? {
                Event::Input(input) => match input {
                    KeyEvent::Char('q') => {
                        // Quit the program
                        break;
                    }
                    KeyEvent::Char('n') => {
                        // Start a new game
                        app.playing = true;
                        game_in_progress = true;
                    }
                    KeyEvent::Up => selected_option = if selected_option > 0 {
                        selected_option - 1
                    } else {
                        menu_items.len() - 1
                    },
                    KeyEvent::Down => selected_option = if selected_option >= menu_items.len() - 1 {
                        0
                    } else {
                        selected_option + 1
                    },
                    _ => {}
                }
                Event::Tick => {}
            }
        }
    }

    Ok(())
}
