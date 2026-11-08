#[derive(Debug, Clone, PartialEq)]
pub enum NxType {
    F32, I32, U32, Bool, Str,
    Vec2, Vec3, Vec4, Mat4, Quat,
    World, EntityId, CommandBuffer,
    Array(Box<NxType>, usize),
    Custom(String),
    Ref(Box<NxType>),
    MutRef(Box<NxType>),
    Void,
}

impl NxType {
    pub fn from_name(name: &str) -> Self {
        match name {
            "f32" => NxType::F32,
            "i32" => NxType::I32,
            "u32" => NxType::U32,
            "bool" => NxType::Bool,
            "str" => NxType::Str,
            "vec2" | "Vec2" => NxType::Vec2,
            "vec3" | "Vec3" => NxType::Vec3,
            "vec4" | "Vec4" => NxType::Vec4,
            "mat4" | "Mat4" => NxType::Mat4,
            "quat" | "Quat" => NxType::Quat,
            "World" => NxType::World,
            "EntityId" => NxType::EntityId,
            "CommandBuffer" => NxType::CommandBuffer,
            other => NxType::Custom(other.to_string()),
        }
    }
    pub fn is_numeric(&self) -> bool { matches!(self, NxType::F32 | NxType::I32 | NxType::U32) }
    pub fn is_vector(&self) -> bool { matches!(self, NxType::Vec2 | NxType::Vec3 | NxType::Vec4) }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FnDecorator { Entry, System, GpuVertex, GpuFragment, GpuCompute, None }

#[derive(Debug, Clone, PartialEq)]
pub enum VarKind { Let, Var, Temp, Const }

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp { Neg, Not }

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    Eq, NotEq, Less, LessEq, Greater, GreaterEq,
    And, Or, Assign, AddAssign, SubAssign, MulAssign, DivAssign,
}

#[derive(Debug, Clone)]
pub enum Expr {
    FloatLit(f32),
    IntLit(i64),
    BoolLit(bool),
    StringLit(String),
    Ident(String),
    ArrayLit(Vec<Expr>),
    IndexAccess { array: Box<Expr>, index: Box<Expr> },
    FieldAccess { base: Box<Expr>, field: String },
    MethodCall { receiver: Box<Expr>, method: String, args: Vec<(Option<String>, Expr)> },
    FnCall { name: String, args: Vec<(Option<String>, Expr)> },
    StructInit { name: String, fields: Vec<(String, Expr)> },
    Unary { op: UnaryOp, expr: Box<Expr> },
    Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    Await(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EcsQueryMode { SingleMut, MutRef, PairMut }

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl { kind: VarKind, name: String, ty: Option<NxType>, value: Expr },
    ExprStmt(Expr),
    IfStmt { condition: Expr, then_branch: Vec<Stmt>, else_branch: Option<Vec<Stmt>> },
    ForRangeStmt { iterator: String, start: Expr, end: Expr, body: Vec<Stmt> },
    ForEcsQueryStmt {
        world_expr: Expr,
        query_mode: EcsQueryMode,
        entity_var: String,
        component_vars: Vec<String>,
        component_types: Vec<NxType>,
        body: Vec<Stmt>,
    },
    WhileStmt { condition: Expr, body: Vec<Stmt> },
    BreakStmt,
    ContinueStmt,
    ReturnStmt(Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub gpu_shared: bool,
    pub is_component: bool,
    pub fields: Vec<(String, NxType)>,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub decorator: FnDecorator,
    pub is_async: bool,
    pub params: Vec<(String, NxType)>,
    pub return_type: NxType,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct NxProgram {
    pub imports: Vec<Vec<String>>,
    pub structs: Vec<StructDef>,
    pub functions: Vec<FnDef>,
}
