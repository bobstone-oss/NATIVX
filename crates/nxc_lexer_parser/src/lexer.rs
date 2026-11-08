use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    #[token("@entry")] AttrEntry,
    #[token("@gpu_shared")] AttrGpuShared,
    #[token("@component")] AttrComponent,
    #[token("@system")] AttrSystem,
    #[token("@gpu_vertex")] AttrGpuVertex,
    #[token("@gpu_fragment")] AttrGpuFragment,
    #[token("@gpu_compute")] AttrGpuCompute,

    #[token("use")] Use,
    #[token("pub")] Pub,
    #[token("struct")] Struct,
    #[token("fn")] Fn,
    #[token("async")] Async,
    #[token("await")] Await,
    #[token("let")] Let,
    #[token("var")] Var,
    #[token("temp")] Temp,
    #[token("const")] Const,
    #[token("if")] If,
    #[token("else")] Else,
    #[token("for")] For,
    #[token("in")] In,
    #[token("while")] While,
    #[token("break")] Break,
    #[token("continue")] Continue,
    #[token("return")] Return,
    #[token("true")] True,
    #[token("false")] False,

    #[token("f32")] TypeF32,
    #[token("i32")] TypeI32,
    #[token("u32")] TypeU32,
    #[token("bool")] TypeBool,
    #[token("str")] TypeStr,
    #[token("vec2")] TypeVec2,
    #[token("vec3")] TypeVec3,
    #[token("vec4")] TypeVec4,
    #[token("mat4")] TypeMat4,
    #[token("quat")] TypeQuat,
    #[token("World")] TypeWorld,
    #[token("EntityId")] TypeEntityId,
    #[token("CommandBuffer")] TypeCommandBuffer,

    #[token("->")] Arrow,
    #[token("..")] DotDot,
    #[token("&mut")] RefMut,
    #[token("&")] Ref,
    #[token("{")] LBrace,
    #[token("}")] RBrace,
    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("[")] LBracket,
    #[token("]")] RBracket,
    #[token(":")] Colon,
    #[token(";")] Semicolon,
    #[token(",")] Comma,
    #[token(".")] Dot,

    #[token("==")] EqEq,
    #[token("!=")] NotEq,
    #[token("<=")] LessEq,
    #[token(">=")] GreaterEq,
    #[token("&&")] AndAnd,
    #[token("||")] OrOr,
    #[token("+=")] PlusAssign,
    #[token("-=")] MinusAssign,
    #[token("*=")] MulAssign,
    #[token("/=")] DivAssign,

    #[token("=")] Assign,
    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token("*")] Star,
    #[token("/")] Slash,
    #[token("%")] Percent,
    #[token("!")] Bang,
    #[token("<")] Less,
    #[token(">")] Greater,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f32>().ok())]
    FloatLit(f32),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(i64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    StringLit(String),
}
