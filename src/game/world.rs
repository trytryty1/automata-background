use rand::Rng;

pub struct World {
    pub size: (usize, usize),
    pub prey_count: u32,
    pub preditor_count: u32,
    pub cells: Vec<Cell>,
}

impl World {
    pub fn new(size: (usize, usize)) -> Self {
        let (rows, cols) = size;
        let mut cells = vec![
            Cell {
                cell_type: CellType::Empty,
                created_at: 0,
            };
            rows as usize * cols as usize
        ];

        Self {
            size,
            cells,
            prey_count: 0,
            preditor_count: 0,
        }
    }

    pub fn seed_preditor_prey(&mut self, ticks: u32) {
        // add 100 random placed preditors
        for _ in 0..100 {
            let mut random_idx;
            loop {
                random_idx = rand::thread_rng().gen_range(0..self.cells.len());
                match self.cells[random_idx].cell_type {
                    CellType::Preditor | CellType::Prey => continue, // Skip and retry
                    _ => break,                                      // Found a valid spot
                }
            }

            self.cells[random_idx] = Cell {
                cell_type: CellType::Preditor,
                created_at: ticks,
            };
        }

        // add 300 random placed prey
        for _ in 0..300 {
            let mut random_idx;
            loop {
                random_idx = rand::thread_rng().gen_range(0..self.cells.len());
                match self.cells[random_idx].cell_type {
                    CellType::Preditor | CellType::Prey => continue, // Skip and retry
                    _ => break,                                      // Found a valid spot
                }
            }

            self.cells[random_idx] = Cell {
                cell_type: CellType::Prey,
                created_at: ticks,
            };
        }
    }

    pub fn clear_cell_types(&mut self) {
        for cell in &mut self.cells {
            cell.cell_type = CellType::Empty;
        }
    }

    pub fn get_cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.size.1 + col]
    }

    pub fn get_mut_cell(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.cells[row * self.size.1 + col]
    }

    pub fn get_cell_x_y(&self, index: usize) -> (usize, usize) {
        (index / self.size.1, index % self.size.1)
    }
}

pub struct Simulation {
    pub worlds: [World; 2],
    pub active_world: usize,
    ticks: u32,
}

impl Simulation {
    pub fn new(size: (usize, usize)) -> Self {
        Self {
            worlds: [World::new(size), World::new(size)],
            active_world: 0,
            ticks: 0,
        }
    }

    pub fn tick(&mut self) {
        self.ticks += 1;
    }

    fn get_active_inactive(&mut self) -> (&World, &mut World) {
        if self.active_world == 0 {
            let (first, second) = self.worlds.split_at_mut(1);
            (&first[0], &mut second[0])
        } else {
            let (first, second) = self.worlds.split_at_mut(1);
            (&second[0], &mut first[0])
        }
    }

    pub fn reset_simulation(&mut self) {
        // reseed the worlds
        self.worlds[0].seed_preditor_prey(self.ticks);
        self.worlds[1].seed_preditor_prey(self.ticks);
    }

    pub fn update(&mut self) {
        let active_idx = self.active_world;
        let inactive_idx = if active_idx == 0 { 1 } else { 0 };

        let ticks = self.ticks;

        // Split mutable references to avoid borrow conflicts
        let (active, inactive) = self.get_active_inactive();
        // Clear inactive world
        inactive.clear_cell_types();

        if ticks == 0 {
            inactive.seed_preditor_prey(ticks);
        }

        for row in 0..active.size.0 {
            for col in 0..active.size.1 {
                let cell = active.get_cell(row as usize, col as usize);

                match cell.cell_type {
                    CellType::Prey => {
                        let mut found = false;
                        let mut tries = 0;
                        let mut neighbor_row = 0;
                        let mut neighbor_col = 0;
                        while !found && tries < 9 {
                            tries += 1;
                            let rand_row = rand::thread_rng().gen_range(0..3) as i32 - 1;
                            let rand_col = rand::thread_rng().gen_range(0..3) as i32 - 1;

                            neighbor_row = ((row as i32 + rand_row + active.size.0 as i32)
                                % active.size.0 as i32)
                                as usize;
                            neighbor_col = ((col as i32 + rand_col + active.size.1 as i32)
                                % active.size.1 as i32)
                                as usize;

                            if neighbor_row < 0
                                || neighbor_col < 0
                                || neighbor_row >= active.size.0
                                || neighbor_col >= active.size.1
                            {
                                continue;
                            }
                            match inactive
                                .get_cell(neighbor_row as usize, neighbor_col as usize)
                                .cell_type
                            {
                                CellType::Empty => {
                                    found = true;
                                }
                                _ => continue,
                            }
                        }

                        if !found {
                            // If it can't find an empty neighbor it will die
                            // inactive.prey_count -= 1;
                            continue;
                        }

                        // The prey will try to reproduce itself every 25 ticks since it was created
                        if (ticks - cell.created_at) % 25 == 0 {
                            inactive
                                .get_mut_cell(neighbor_row as usize, neighbor_col as usize)
                                .cell_type = CellType::Prey;
                            inactive
                                .get_mut_cell(neighbor_row as usize, neighbor_col as usize)
                                .created_at = ticks;
                            inactive.prey_count += 1;

                            // copy itself to the new cell
                            inactive.get_mut_cell(row, col).cell_type = CellType::Prey;
                            inactive.get_mut_cell(row, col).created_at = cell.created_at;

                            continue;
                        }

                        // The prey will move to a random empty neighbor
                        inactive
                            .get_mut_cell(neighbor_row as usize, neighbor_col as usize)
                            .cell_type = CellType::Prey;
                        continue;
                    }

                    CellType::Preditor => {
                        // If the preditor has been alive for 55 ticks it will die
                        if (ticks - cell.created_at) > 55 {
                            // inactive.preditor_count -= 1;
                            continue;
                        }

                        // The preditor will look in one spot
                        // If it sees a prey it will convert it to a predator
                        // If it sees an empty spot it will move to it
                        // If it sees a predator it will not move
                        let mut found = false;
                        let mut tries = 0;
                        let mut neighbor_row = 0;
                        let mut neighbor_col = 0;
                        while !found && tries < 9 {
                            tries += 1;
                            let rand_row = rand::thread_rng().gen_range(0..3) as i32 - 1;
                            let rand_col = rand::thread_rng().gen_range(0..3) as i32 - 1;

                            neighbor_row = ((row as i32 + rand_row + active.size.0 as i32)
                                % active.size.0 as i32)
                                as usize;
                            neighbor_col = ((col as i32 + rand_col + active.size.1 as i32)
                                % active.size.1 as i32)
                                as usize;

                            if neighbor_row < 0
                                || neighbor_col < 0
                                || neighbor_row >= active.size.0
                                || neighbor_col >= active.size.1
                            {
                                continue;
                            }
                            match inactive
                                .get_cell(neighbor_row as usize, neighbor_col as usize)
                                .cell_type
                            {
                                CellType::Empty | CellType::Prey => {
                                    found = true;
                                }
                                _ => continue,
                            }
                        }

                        if !found {
                            // If it can't find an empty neighbor it will die
                            // inactive.preditor_count -= 1;
                            continue;
                        }

                        match inactive
                            .get_cell(neighbor_row as usize, neighbor_col as usize)
                            .cell_type
                        {
                            CellType::Prey => {
                                // inactive.preditor_count += 1;
                                // inactive.prey_count -= 1;
                                inactive
                                    .get_mut_cell(neighbor_row as usize, neighbor_col as usize)
                                    .cell_type = CellType::Preditor;
                                inactive
                                    .get_mut_cell(neighbor_row as usize, neighbor_col as usize)
                                    .created_at = ticks;

                                inactive.get_mut_cell(row, col).cell_type = CellType::Preditor;
                                inactive.get_mut_cell(row, col).created_at = cell.created_at;

                                continue;
                            }
                            CellType::Preditor => {
                                // inactive.preditor_count -= 1;
                            }
                            CellType::Empty => {
                                inactive
                                    .get_mut_cell(neighbor_row as usize, neighbor_col as usize)
                                    .cell_type = CellType::Preditor;
                                inactive
                                    .get_mut_cell(neighbor_row as usize, neighbor_col as usize)
                                    .created_at = cell.created_at;
                            }
                        }
                    }
                    CellType::Empty => continue,
                }
            }
        }

        self.active_world = inactive_idx;

        self.tick();
    }
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub cell_type: CellType,
    pub created_at: u32,
}

#[derive(Clone, Copy)]
pub enum CellType {
    Empty,
    Preditor,
    Prey,
}
