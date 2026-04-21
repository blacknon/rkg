#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Receiver {
    Rec,
    Grid,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub receiver: Receiver,
    pub calls: Vec<Call>,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub stages: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Str(String),
    Num(i64),
    Ident(String),
    Call(Call),
}

#[derive(Debug, Clone)]
pub struct RecConfig {
    pub fs: String,
    pub rs: String,
    pub ofs: String,
    pub ors: String,
}

impl Default for RecConfig {
    fn default() -> Self {
        Self {
            fs: r"\s+".to_string(),
            rs: "\n".to_string(),
            ofs: " ".to_string(),
            ors: "\n".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GridConfig {
    pub fs: Option<String>,
    pub rs: String,
    pub ofs: String,
    pub ors: String,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            fs: None,
            rs: "\n".to_string(),
            ofs: "".to_string(),
            ors: "\n".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Records {
    pub rows: Vec<Vec<String>>,
    pub cfg: RecConfig,
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub cells: Vec<Vec<String>>,
    pub cfg: GridConfig,
}

#[derive(Debug, Clone)]
pub enum Agg {
    Sum(usize),
    Count,
    Min(usize),
    Max(usize),
    Avg(usize),
    Median(usize),
}
