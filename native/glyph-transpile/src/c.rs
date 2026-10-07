use glyph_frontend::{Body, Block, Expr, Func, Global, Program, Stmt};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct COptions {
    pub rt_header: String,
    pub net_header: String,
}

impl Default for COptions {
    fn default() -> Self {
        Self {
            rt_header: "../../runtime/glyph_rt.h".into(),
            net_header: "../../runtime/glyph_net.h".into(),
        }
    }
}

pub fn emit_c(prog: &Program, opts: &COptions) -> anyhow::Result<String> {
    Ok(CGen::new(prog, opts).gen())
}

struct CGen<'a> {
    prog: &'a Program,
    opts: &'a COptions,
    byte_globals: HashSet<String>,
    global_sizes: HashMap<String, u32>,
    func_types: HashMap<String, (String, Vec<(String, String)>)>,
}

impl<'a> CGen<'a> {
    fn new(prog: &'a Program, opts: &'a COptions) -> Self {
        let byte_globals: HashSet<_> = prog
            .globals
            .iter()
            .filter(|g| g.byte)
            .map(|g| g.name.clone())
            .collect();
        let global_sizes: HashMap<_, _> = prog
            .globals
            .iter()
            .map(|g| (g.name.clone(), g.size))
            .collect();
        let mut func_types = HashMap::new();
        for f in &prog.funcs {
            let ps: Vec<_> = f
                .params
                .iter()
                .map(|p| (p.name.clone(), c_type(p.typ.as_deref().unwrap_or("i"))))
                .collect();
            let mut rt = if f.ret.as_deref() == Some("v") {
                "void".into()
            } else {
                c_type(f.ret.as_deref().unwrap_or("i"))
            };
            if f.name == "main" || f.ret.as_deref() == Some("v") {
                if f.name == "main" {
                    rt = "int".into();
                } else if f.ret.as_deref() == Some("v") {
                    rt = "void".into();
                }
            }
            func_types.insert(f.name.clone(), (rt, ps));
        }
        Self {
            prog,
            opts,
            byte_globals,
            global_sizes,
            func_types,
        }
    }

    fn gen(&self) -> String {
        let mut lines = vec![
            format!("#include \"{}\"", self.opts.rt_header),
            format!("#include \"{}\"", self.opts.net_header),
            "#include <stdint.h>".into(),
            String::new(),
            "int glyph_argc_val;".into(),
            "char **glyph_argv_val;".into(),
            String::new(),
        ];
        for g in &self.prog.globals {
            lines.push(self.gen_global(g));
            lines.push(String::new());
        }
        for f in &self.prog.funcs {
            lines.push(self.gen_func(f));
        }
        if !self.prog.funcs.iter().any(|f| f.name == "main") {
            lines.push("int main(int argc, char **argv) {".into());
            lines.push("    glyph_argc_val = argc; glyph_argv_val = argv;".into());
            lines.push("    return 0;".into());
            lines.push("}".into());
        }
        lines.join("\n") + "\n"
    }

    fn gen_global(&self, g: &Global) -> String {
        if g.byte {
            format!("static char {}[{}];", g.name, g.size)
        } else {
            format!("static int64_t {}[{}];", g.name, g.size)
        }
    }

    fn gen_func(&self, f: &Func) -> String {
        let (rt, _) = self.func_types.get(&f.name).cloned().unwrap_or_else(|| {
            ("int64_t".into(), Vec::new())
        });
        let is_main = f.name == "main";
        let params = f
            .params
            .iter()
            .map(|p| format!("{} {}", c_type(p.typ.as_deref().unwrap_or("i")), p.name))
            .collect::<Vec<_>>()
            .join(", ");
        match &f.body {
            Body::Block(block) => {
                let ret = if is_main { "void" } else { rt.as_str() };
                let mut body = self.gen_block(block, 1, ret);
                if is_main {
                    let prelude = "    glyph_argc_val = argc;\n    glyph_argv_val = argv;\n";
                    body = format!("{prelude}{body}    return 0;\n");
                    format!("int main(int argc, char **argv) {{\n{body}}}\n")
                } else {
                    format!("{rt} {name}({params}) {{\n{body}}}\n", name = f.name)
                }
            }
            Body::Expr(expr) => {
                let expr = self.gen_expr(expr);
                if rt == "void" {
                    format!("{rt} {name}({params}) {{ {expr}; }}\n", name = f.name)
                } else {
                    format!("{rt} {name}({params}) {{ return {expr}; }}\n", name = f.name)
                }
            }
        }
    }

    fn gen_block(&self, block: &Block, indent: usize, ret: &str) -> String {
        let pad = "    ".repeat(indent);
        let mut lines = Vec::new();
        for s in &block.stmts {
            match s {
                Stmt::Let { name, expr } => {
                    lines.push(format!("{pad}int64_t {name} = {};", self.gen_expr(expr)));
                }
                Stmt::While { cond, body } => {
                    lines.push(format!("{pad}while ({}) {{", self.gen_expr(cond)));
                    lines.push(self.gen_block(body, indent + 1, ret));
                    lines.push(format!("{pad}}}"));
                }
                Stmt::Return(expr) => {
                    let e = self.gen_expr(expr);
                    if ret == "void" {
                        lines.push(format!("{pad}(void)({e}); return;"));
                    } else {
                        lines.push(format!("{pad}return {e};"));
                    }
                }
                Stmt::Expr(expr) => {
                    if let Expr::Ternary { .. } = expr {
                        if branch_is_stmt_form(expr) {
                            lines.extend(self.gen_if_else(expr, indent));
                            continue;
                        }
                    }
                    if let Expr::IndexAssign { name, idx, expr: val } = expr {
                        lines.push(format!("{pad}{};", self.gen_index_assign(name, idx, val)));
                        continue;
                    }
                    let e = self.gen_expr(expr);
                    if ret != "void" {
                        lines.push(format!("{pad}(void)({e});"));
                    } else {
                        lines.push(format!("{pad}{e};"));
                    }
                }
            }
        }
        let mut out = lines.join("\n");
        if !out.is_empty() {
            out.push('\n');
        }
        out
    }

    fn gen_if_else(&self, expr: &Expr, indent: usize) -> Vec<String> {
        let Expr::Ternary { cond, then_, else_ } = expr else {
            return vec![];
        };
        let pad = "    ".repeat(indent);
        let mut lines = vec![format!("{pad}if ({}) {{", self.gen_expr(cond))];
        lines.extend(self.gen_branch_lines(then_, indent + 1));
        lines.push(format!("{pad}}} else {{"));
        lines.extend(self.gen_branch_lines(else_, indent + 1));
        lines.push(format!("{pad}}}"));
        lines
    }

    fn gen_branch_lines(&self, expr: &Expr, indent: usize) -> Vec<String> {
        let pad = "    ".repeat(indent);
        match expr {
            Expr::Ternary { .. } if branch_is_stmt_form(expr) => self.gen_if_else(expr, indent),
            Expr::Block(b) => {
                let blk = self.gen_block(b, indent, "void");
                blk.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.to_string())
                    .collect()
            }
            Expr::IndexAssign { name, idx, expr: val } => {
                vec![format!("{pad}{};", self.gen_index_assign(name, idx, val))]
            }
            Expr::Assign { name, expr: val } => {
                vec![format!("{pad}{};", self.gen_expr(&Expr::Assign {
                    name: name.clone(),
                    expr: val.clone(),
                }))]
            }
            other => vec![format!("{pad}{};", self.gen_expr(other))],
        }
    }

    fn gen_index_assign(&self, name: &str, idx: &Expr, val: &Expr) -> String {
        let v = self.gen_expr(val);
        let val = if self.byte_globals.contains(name) {
            format!("(char)({v})")
        } else {
            v
        };
        format!("{name}[{}] = {val}", self.gen_expr(idx))
    }

    fn gen_expr(&self, e: &Expr) -> String {
        match e {
            Expr::Num(n) => {
                if n.contains('.') {
                    n.clone()
                } else {
                    n.clone()
                }
            }
            Expr::Str(s) => format!("\"{}\"", escape_c_string(s)),
            Expr::Var(name) => name.clone(),
            Expr::BinOp { op, left, right } => {
                format!(
                    "({} {} {})",
                    self.gen_expr(left),
                    op,
                    self.gen_expr(right)
                )
            }
            Expr::Unary { op, expr } => {
                if op == "!" {
                    format!("(!{})", self.gen_expr(expr))
                } else {
                    format!("(-{})", self.gen_expr(expr))
                }
            }
            Expr::Ternary { cond, then_, else_ } => {
                format!(
                    "({} ? {} : {})",
                    self.gen_expr(cond),
                    self.gen_expr(then_),
                    self.gen_expr(else_)
                )
            }
            Expr::Call { name, args } => self.gen_call(name, args),
            Expr::Index { name, idx } => {
                if self.byte_globals.contains(name) {
                    format!(
                        "(int64_t)(unsigned char){name}[{}]",
                        self.gen_expr(idx)
                    )
                } else {
                    format!("{name}[{}]", self.gen_expr(idx))
                }
            }
            Expr::Assign { name, expr } => {
                format!("({name} = {})", self.gen_expr(expr))
            }
            Expr::IndexAssign { name, idx, expr } => self.gen_index_assign(name, idx, expr),
            Expr::Block(b) => {
                let inner = self
                    .gen_block(b, 0, "void")
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("({{ {inner} }})")
            }
        }
    }

    fn gen_call(&self, name: &str, args: &[Expr]) -> String {
        match name {
            "p" => format!("glyph_print_i({})", self.gen_expr(&args[0])),
            "pf" => format!("glyph_print_f({})", self.gen_expr(&args[0])),
            "ps" => format!("glyph_print_s({})", self.gen_expr(&args[0])),
            "o" => format!("glyph_putc({})", self.gen_expr(&args[0])),
            "r" => "glyph_getc()".into(),
            "len" => format!("glyph_strlen({})", self.gen_expr(&args[0])),
            "ch" => format!(
                "glyph_char_at({}, {})",
                self.gen_expr(&args[0]),
                self.gen_expr(&args[1])
            ),
            "argc" => "glyph_argc()".into(),
            "argv" => format!("glyph_arg({})", self.gen_expr(&args[0])),
            "tcpconn" => format!(
                "glyph_tcp_connect({}, {})",
                self.gen_expr(&args[0]),
                self.gen_expr(&args[1])
            ),
            "tcpsend" => format!(
                "glyph_tcp_send({}, {}, {})",
                self.gen_expr(&args[0]),
                self.gen_expr(&args[1]),
                self.gen_expr(&args[2])
            ),
            "tcpread" => format!(
                "glyph_tcp_read({}, {}, {})",
                self.gen_expr(&args[0]),
                self.gen_expr(&args[1]),
                self.gen_expr(&args[2])
            ),
            "tcpclose" => format!("glyph_tcp_close({})", self.gen_expr(&args[0])),
            "stdout" => format!(
                "glyph_write_stdout({}, {})",
                self.gen_expr(&args[0]),
                self.gen_expr(&args[1])
            ),
            "parseurl" => {
                let hs = self.global_sizes.get("host").copied().unwrap_or(256);
                let ps = self.global_sizes.get("path").copied().unwrap_or(2048);
                format!(
                    "glyph_parse_http_url({}, host, {}, path, {})",
                    self.gen_expr(&args[0]),
                    hs,
                    ps
                )
            }
            "body" => format!("glyph_http_read_body({})", self.gen_expr(&args[0])),
            _ => {
                let a = args.iter().map(|x| self.gen_expr(x)).collect::<Vec<_>>().join(", ");
                format!("{name}({a})")
            }
        }
    }
}

fn c_type(t: &str) -> String {
    match t {
        "i" => "int64_t".into(),
        "f" => "double".into(),
        "b" => "int".into(),
        "v" => "void".into(),
        "s" => "const char*".into(),
        _ => "int64_t".into(),
    }
}

fn escape_c_string(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn branch_is_stmt_form(expr: &Expr) -> bool {
    match expr {
        Expr::Block(_) => true,
        Expr::Ternary { then_, else_, .. } => {
            branch_is_stmt_form(then_) || branch_is_stmt_form(else_)
        }
        _ => false,
    }
}
