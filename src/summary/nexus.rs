use std::collections::HashMap;
use std::io::BufRead;

use super::{Error, Result};

const MAX_STATEMENT: usize = 2 * 1024 * 1024;
const MAX_TAXA: usize = 4096;

#[derive(Clone, Debug)]
pub struct Node {
    pub children: Option<[usize; 2]>,
    pub tip: Option<usize>,
    pub parent: Option<usize>,
    pub length: f64,
    pub height: f64,
    pub rate: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Tree {
    pub state: u64,
    pub nodes: Vec<Node>,
    pub root: usize,
    pub tips: Vec<usize>,
}

pub type Clade = Vec<u64>;

impl Tree {
    pub fn clades(&self) -> Vec<Clade> {
        let mut masks = vec![vec![0; self.tips.len().div_ceil(64)]; self.nodes.len()];
        for (i, node) in self.nodes.iter().enumerate() {
            if let Some(tip) = node.tip {
                masks[i][tip / 64] |= 1 << (tip % 64);
            } else if let Some([a, b]) = node.children {
                for word in 0..masks[i].len() {
                    masks[i][word] = masks[a][word] | masks[b][word];
                }
            }
        }
        masks
    }

    pub fn ancestor(&self, clade: &[u64], masks: &[Clade]) -> usize {
        let word = clade.iter().position(|&x| x != 0).expect("nonempty clade");
        let tip = word * 64 + clade[word].trailing_zeros() as usize;
        let mut node = self.tips[tip];
        while !clade.iter().zip(&masks[node]).all(|(a, b)| a & b == *a) {
            node = self.nodes[node].parent.expect("root contains every tip");
        }
        node
    }
}

pub struct Reader<R> {
    input: R,
    pub taxa: Vec<String>,
    translation: HashMap<String, usize>,
    block: String,
    trees_seen: bool,
    declared_taxa: Option<usize>,
    ended: bool,
    last_state: Option<u64>,
    rates: Option<bool>,
}

impl<R: BufRead> Reader<R> {
    pub fn new(mut input: R) -> Result<Self> {
        let mut header = [0; 6];
        std::io::Read::read_exact(&mut input, &mut header)?;
        if !header.eq_ignore_ascii_case(b"#NEXUS") {
            return Err(Error("expected a NEXUS tree file".into()));
        }
        Ok(Self {
            input,
            taxa: Vec::new(),
            translation: HashMap::new(),
            block: String::new(),
            trees_seen: false,
            declared_taxa: None,
            ended: false,
            last_state: None,
            rates: None,
        })
    }

    pub fn next_tree(&mut self) -> Result<Option<Tree>> {
        while let Some(statement) = statement(&mut self.input)? {
            let mut p = Tokens::new(&statement);
            let command = p.word()?.to_ascii_lowercase();
            match command.as_str() {
                "begin" => {
                    if !self.block.is_empty() || self.ended {
                        return Err(Error("unexpected NEXUS block".into()));
                    }
                    self.block = p.word()?.to_ascii_lowercase();
                    if !p.done()? {
                        return Err(Error("trailing BEGIN content".into()));
                    }
                    if !["taxa", "trees"].contains(&self.block.as_str()) {
                        return Err(Error("only TAXA and TREES blocks are supported".into()));
                    }
                    if self.block == "trees" {
                        if self.trees_seen {
                            return Err(Error("multiple TREES blocks are not supported".into()));
                        }
                        self.trees_seen = true;
                    }
                }
                "end" | "endblock" => {
                    if !p.done()? {
                        return Err(Error("trailing END content".into()));
                    }
                    if self.block == "trees" {
                        self.ended = true;
                    }
                    if self.block.is_empty() {
                        return Err(Error("END outside a block".into()));
                    }
                    self.block.clear();
                }
                "dimensions" if self.block == "taxa" => {
                    if self.declared_taxa.is_some() || !p.word()?.eq_ignore_ascii_case("ntax") {
                        return Err(Error("expected one NTAX dimension".into()));
                    }
                    p.expect(b'=')?;
                    let count = p
                        .word()?
                        .parse()
                        .map_err(|_| Error("invalid NTAX".into()))?;
                    if !(2..=MAX_TAXA).contains(&count) || !p.done()? {
                        return Err(Error("invalid TAXA dimensions".into()));
                    }
                    self.declared_taxa = Some(count);
                }
                "taxlabels" if self.block == "taxa" => {
                    if !self.taxa.is_empty() {
                        return Err(Error("duplicate TAXLABELS".into()));
                    }
                    while !p.done()? {
                        self.taxa.push(p.label()?);
                    }
                    check_taxa(&self.taxa)?;
                }
                "translate" if self.block == "trees" && self.last_state.is_none() => {
                    if !self.translation.is_empty() {
                        return Err(Error("duplicate TRANSLATE table".into()));
                    }
                    let mut names = Vec::new();
                    while !p.done()? {
                        let key = p.word()?;
                        let name = p.label()?;
                        if self.translation.insert(key, names.len()).is_some() {
                            return Err(Error("duplicate translation key".into()));
                        }
                        names.push(name);
                        if !p.done()? {
                            p.expect(b',')?;
                        }
                    }
                    check_taxa(&names)?;
                    if self.declared_taxa.is_some_and(|n| n != names.len()) {
                        return Err(Error("NTAX and TRANSLATE disagree".into()));
                    }
                    if !self.taxa.is_empty()
                        && (self.taxa.len() != names.len()
                            || !self.taxa.iter().all(|name| names.contains(name)))
                    {
                        return Err(Error("TAXLABELS and TRANSLATE disagree".into()));
                    }
                    self.taxa = names;
                }
                "tree" if self.block == "trees" => {
                    if self.ended || self.translation.is_empty() {
                        return Err(Error("trees require a TRANSLATE table".into()));
                    }
                    let name = p.word()?;
                    let state: u64 = name
                        .strip_prefix("STATE_")
                        .ok_or_else(|| Error("expected a STATE_<integer> sample name".into()))?
                        .parse()
                        .map_err(|_| Error("invalid sample state".into()))?;
                    if self.last_state.is_some_and(|last| state <= last) {
                        return Err(Error("sample states must increase strictly".into()));
                    }
                    p.expect(b'=')?;
                    let prefix = p.annotations()?;
                    if prefix.iter().any(|a| a.trim().eq_ignore_ascii_case("&U")) {
                        return Err(Error("unrooted trees are not supported".into()));
                    }
                    let mut nodes = Vec::new();
                    let mut tips = vec![usize::MAX; self.taxa.len()];
                    let root = p.node(&self.translation, &mut nodes, &mut tips, 0)?;
                    if !p.done()? || tips.contains(&usize::MAX) {
                        return Err(Error("tree has missing taxa or trailing content".into()));
                    }
                    if nodes[root].length != 0.0 || nodes[root].rate.is_some() {
                        return Err(Error("root stems and root rates are not supported".into()));
                    }
                    let rated = nodes
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != root)
                        .filter(|(_, n)| n.rate.is_some())
                        .count();
                    if rated != 0 && rated != nodes.len() - 1 {
                        return Err(Error(
                            "branch rate annotations must cover every non-root branch".into(),
                        ));
                    }
                    let has_rates = rated > 0;
                    if self.rates.is_some_and(|v| v != has_rates) {
                        return Err(Error("mixed clock annotations in the tree sample".into()));
                    }
                    self.rates = Some(has_rates);
                    self.last_state = Some(state);
                    return Ok(Some(Tree {
                        state,
                        nodes,
                        root,
                        tips,
                    }));
                }
                _ => {
                    return Err(Error(format!(
                        "unsupported NEXUS command {command:?} in {}",
                        self.block
                    )));
                }
            }
        }
        if !self.ended || !self.block.is_empty() || self.last_state.is_none() {
            return Err(Error(
                "incomplete tree file: missing trees or final END;".into(),
            ));
        }
        Ok(None)
    }
}

fn check_taxa(taxa: &[String]) -> Result<()> {
    let unique: std::collections::HashSet<_> = taxa.iter().collect();
    if taxa.len() < 2
        || taxa.len() > MAX_TAXA
        || unique.len() != taxa.len()
        || taxa.iter().any(|t| t.is_empty())
    {
        return Err(Error("expected 2–4096 distinct, nonempty taxa".into()));
    }
    Ok(())
}

// Statements are bounded independently of file size; semicolons inside comments/labels do not end them.
fn statement(input: &mut impl BufRead) -> Result<Option<String>> {
    let mut out = Vec::new();
    let mut quote = false;
    let mut comment = 0usize;
    loop {
        let bytes = input.fill_buf()?;
        if bytes.is_empty() {
            let text =
                std::str::from_utf8(&out).map_err(|_| Error("tree file must be UTF-8".into()))?;
            if !quote && comment == 0 && Tokens::new(text).done()? {
                return Ok(None);
            }
            return Err(Error("truncated NEXUS statement".into()));
        }
        let c = bytes[0];
        input.consume(1);
        if comment > 0 {
            if c == b'[' {
                comment += 1;
            }
            if c == b']' {
                comment -= 1;
            }
        } else if c == b'\'' {
            quote = !quote;
        } else if !quote && c == b'[' {
            comment = 1;
        } else if !quote && c == b';' {
            return String::from_utf8(out)
                .map(Some)
                .map_err(|_| Error("tree file must be UTF-8".into()));
        }
        out.push(c);
        if out.len() > MAX_STATEMENT {
            return Err(Error("NEXUS statement exceeds 2 MiB".into()));
        }
    }
}

struct Tokens<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Tokens<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            offset: 0,
        }
    }
    fn whitespace(&mut self) {
        while self
            .bytes
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
    }
    fn annotations(&mut self) -> Result<Vec<String>> {
        let mut values = Vec::new();
        self.whitespace();
        while self.bytes.get(self.offset) == Some(&b'[') {
            self.offset += 1;
            let start = self.offset;
            let mut depth = 1;
            while depth > 0 {
                let c = *self
                    .bytes
                    .get(self.offset)
                    .ok_or_else(|| Error("unterminated annotation".into()))?;
                self.offset += 1;
                if c == b'[' {
                    depth += 1;
                } else if c == b']' {
                    depth -= 1;
                }
            }
            values.push(String::from_utf8_lossy(&self.bytes[start..self.offset - 1]).into_owned());
            self.whitespace();
        }
        Ok(values)
    }
    fn done(&mut self) -> Result<bool> {
        self.annotations()?;
        Ok(self.offset == self.bytes.len())
    }
    fn expect(&mut self, c: u8) -> Result<()> {
        self.whitespace();
        if self.bytes.get(self.offset) != Some(&c) {
            return Err(Error(format!(
                "expected {:?} near byte {}",
                c as char, self.offset
            )));
        }
        self.offset += 1;
        Ok(())
    }
    fn word(&mut self) -> Result<String> {
        self.annotations()?;
        self.whitespace();
        if self.bytes.get(self.offset) == Some(&b'\'') {
            self.offset += 1;
            let mut word = Vec::new();
            loop {
                let c = *self
                    .bytes
                    .get(self.offset)
                    .ok_or_else(|| Error("unterminated label".into()))?;
                self.offset += 1;
                if c == b'\'' {
                    if self.bytes.get(self.offset) == Some(&c) {
                        self.offset += 1;
                        word.push(c);
                    } else {
                        break;
                    }
                } else {
                    word.push(c);
                }
            }
            return String::from_utf8(word).map_err(|_| Error("invalid UTF-8 label".into()));
        }
        let start = self.offset;
        while self
            .bytes
            .get(self.offset)
            .is_some_and(|c| !c.is_ascii_whitespace() && !b"(),:;=[]".contains(c))
        {
            self.offset += 1;
        }
        if start == self.offset {
            return Err(Error(format!("expected a value near byte {}", start)));
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.offset]).into_owned())
    }
    fn label(&mut self) -> Result<String> {
        self.word().map(|w| w.replace('_', " "))
    }
    fn node(
        &mut self,
        names: &HashMap<String, usize>,
        nodes: &mut Vec<Node>,
        tips: &mut [usize],
        depth: usize,
    ) -> Result<usize> {
        if depth > 512 {
            return Err(Error("tree nesting exceeds 512 levels".into()));
        }
        self.whitespace();
        let (children, tip, height) = if self.bytes.get(self.offset) == Some(&b'(') {
            self.offset += 1;
            let a = self.node(names, nodes, tips, depth + 1)?;
            self.expect(b',')?;
            let b = self.node(names, nodes, tips, depth + 1)?;
            self.expect(b')')?;
            let x = nodes[a].height + nodes[a].length;
            let y = nodes[b].height + nodes[b].length;
            if (x - y).abs() > 1e-9 * x.abs().max(y.abs()).max(1e-12) {
                return Err(Error(
                    "only ultrametric trees with contemporaneous tips are supported".into(),
                ));
            }
            (Some([a, b]), None, x.max(y))
        } else {
            let name = self.word()?;
            let index = *names
                .get(&name)
                .ok_or_else(|| Error(format!("unknown translated taxon {name:?}")))?;
            if tips[index] != usize::MAX {
                return Err(Error("duplicate terminal in tree".into()));
            }
            (None, Some(index), 0.0)
        };
        let mut rate = None;
        let annotations = self.annotations()?;
        for annotation in annotations {
            if let Some(fields) = annotation.strip_prefix('&') {
                for field in fields.split(',') {
                    if let Some(value) = field.trim().strip_prefix("rate=") {
                        if rate.is_some() {
                            return Err(Error("duplicate rate annotation".into()));
                        }
                        let r: f64 = value
                            .parse()
                            .map_err(|_| Error("invalid branch rate".into()))?;
                        if !r.is_finite() || r <= 0.0 {
                            return Err(Error("branch rates must be finite and positive".into()));
                        }
                        rate = Some(r);
                    }
                }
            }
        }
        self.whitespace();
        let length = if self.bytes.get(self.offset) == Some(&b':') {
            self.offset += 1;
            let x: f64 = self
                .word()?
                .parse()
                .map_err(|_| Error("invalid branch length".into()))?;
            if !x.is_finite() || x < 0.0 {
                return Err(Error(
                    "branch lengths must be finite and nonnegative".into(),
                ));
            }
            x
        } else if depth == 0 {
            0.0
        } else {
            return Err(Error("missing branch length".into()));
        };
        if !height.is_finite() {
            return Err(Error("non-finite tree height".into()));
        }
        let id = nodes.len();
        nodes.push(Node {
            children,
            tip,
            parent: None,
            length,
            height,
            rate,
        });
        if let Some([a, b]) = children {
            nodes[a].parent = Some(id);
            nodes[b].parent = Some(id);
        }
        if let Some(tip) = tip {
            tips[tip] = id;
        }
        Ok(id)
    }
}
