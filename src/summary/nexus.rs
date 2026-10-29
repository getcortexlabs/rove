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
    pub state: Option<u64>,
    pub annotations: Vec<NodeAnnotations>,
    pub nodes: Vec<Node>,
    pub root: usize,
    pub tips: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct NodeAnnotations {
    pub posterior: Option<f64>,
    pub constrained: Option<bool>,
    pub height: Option<f64>,
    pub height_mean: Option<f64>,
    pub height_range: Option<[f64; 2]>,
    pub ancestor_mean: Option<f64>,
    pub ancestor_range: Option<[f64; 2]>,
    pub rate_range: Option<[f64; 2]>,
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
    annotated: bool,
    tree_count: usize,
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
            annotated: false,
            tree_count: 0,
        })
    }

    /// Read a single Rove MCC tree, retaining its annotations. Raw samples are rejected.
    pub fn annotated(input: R) -> Result<Self> {
        let mut reader = Self::new(input)?;
        reader.annotated = true;
        Ok(reader)
    }

    pub fn next_tree(&mut self) -> Result<Option<Tree>> {
        let limit = if self.annotated {
            16 * 1024 * 1024
        } else {
            MAX_STATEMENT
        };
        while let Some(statement) = statement(&mut self.input, limit)? {
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
                "translate" if self.block == "trees" && self.tree_count == 0 => {
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
                    let state = if self.annotated {
                        if name != "MCC" || self.tree_count != 0 {
                            return Err(Error("expected exactly one MCC summary tree".into()));
                        }
                        None
                    } else {
                        let value: u64 = name
                            .strip_prefix("STATE_")
                            .ok_or_else(|| Error("expected a STATE_<integer> sample name".into()))?
                            .parse()
                            .map_err(|_| Error("invalid sample state".into()))?;
                        if self.last_state.is_some_and(|last| value <= last) {
                            return Err(Error("sample states must increase strictly".into()));
                        }
                        Some(value)
                    };
                    p.expect(b'=')?;
                    let prefix = p.annotations()?;
                    if prefix.iter().any(|a| a.trim().eq_ignore_ascii_case("&U")) {
                        return Err(Error("unrooted trees are not supported".into()));
                    }
                    if self.annotated && !prefix.iter().any(|a| a.trim().eq_ignore_ascii_case("&R"))
                    {
                        return Err(Error(
                            "summary requires an explicit rooted annotation".into(),
                        ));
                    }
                    let mut annotations = self.annotated.then(Vec::new);
                    let mut nodes = Vec::new();
                    let mut tips = vec![usize::MAX; self.taxa.len()];
                    let root = p.node(
                        &self.translation,
                        &mut nodes,
                        &mut tips,
                        &mut annotations,
                        0,
                    )?;
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
                    if !self.annotated && rated != 0 && rated != nodes.len() - 1 {
                        return Err(Error(
                            "branch rate annotations must cover every non-root branch".into(),
                        ));
                    }
                    let has_rates = rated > 0;
                    if self.rates.is_some_and(|v| v != has_rates) {
                        return Err(Error("mixed clock annotations in the tree sample".into()));
                    }
                    self.rates = Some(has_rates);
                    self.last_state = state;
                    self.tree_count += 1;
                    return Ok(Some(Tree {
                        state,
                        annotations: annotations.unwrap_or_default(),
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
        if !self.ended || !self.block.is_empty() || self.tree_count == 0 {
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
fn statement(input: &mut impl BufRead, limit: usize) -> Result<Option<String>> {
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
        if out.len() > limit {
            return Err(Error(
                "NEXUS statement exceeds the reader size limit".into(),
            ));
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
        metadata: &mut Option<Vec<NodeAnnotations>>,
        depth: usize,
    ) -> Result<usize> {
        if depth > 512 {
            return Err(Error("tree nesting exceeds 512 levels".into()));
        }
        self.whitespace();
        let (children, tip, height) = if self.bytes.get(self.offset) == Some(&b'(') {
            self.offset += 1;
            let a = self.node(names, nodes, tips, metadata, depth + 1)?;
            self.expect(b',')?;
            let b = self.node(names, nodes, tips, metadata, depth + 1)?;
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
        for annotation in &annotations {
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
        if let Some(values) = metadata {
            let annotation = NodeAnnotations::parse(&annotations)?;
            if annotation
                .height
                .is_some_and(|h| (h - height).abs() > 1e-9 * height.abs().max(1e-12))
            {
                return Err(Error(
                    "annotated height disagrees with branch lengths".into(),
                ));
            }
            values.push(annotation);
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

impl NodeAnnotations {
    fn parse(annotations: &[String]) -> Result<Self> {
        let mut result = Self::default();
        let mut seen = std::collections::HashSet::new();
        for annotation in annotations {
            let Some(text) = annotation.strip_prefix('&') else {
                continue;
            };
            let mut start = 0;
            let mut depth = 0usize;
            for (i, c) in text
                .char_indices()
                .chain(std::iter::once((text.len(), ',')))
            {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth = depth
                            .checked_sub(1)
                            .ok_or_else(|| Error("invalid annotation range".into()))?
                    }
                    ',' if depth == 0 => {
                        let field = &text[start..i];
                        if let Some((key, value)) = field.split_once('=') {
                            let key = key.trim();
                            let value = value.trim();
                            if !seen.insert(key.to_owned()) {
                                return Err(Error("duplicate node annotation".into()));
                            }
                            match key {
                                "posterior" => {
                                    let p = number(value)?;
                                    if p > 1.0 {
                                        return Err(Error(
                                            "posterior support must be between zero and one".into(),
                                        ));
                                    }
                                    result.posterior = Some(p);
                                }
                                "constrained" => {
                                    result.constrained = Some(match value {
                                        "true" => true,
                                        "false" => false,
                                        _ => {
                                            return Err(Error(
                                                "invalid constraint annotation".into(),
                                            ));
                                        }
                                    })
                                }
                                "height" => result.height = Some(number(value)?),
                                "height_mean" => result.height_mean = Some(number(value)?),
                                "height_range" => result.height_range = Some(range(value)?),
                                "CAheight_mean" => result.ancestor_mean = Some(number(value)?),
                                "CAheight_range" => result.ancestor_range = Some(range(value)?),
                                "rate_range" => result.rate_range = Some(range(value)?),
                                _ => {}
                            }
                        }
                        start = i + 1;
                    }
                    _ => {}
                }
            }
            if depth != 0 {
                return Err(Error("unterminated annotation range".into()));
            }
        }
        Ok(result)
    }
}
fn number(value: &str) -> Result<f64> {
    let value: f64 = value
        .parse()
        .map_err(|_| Error("invalid numeric annotation".into()))?;
    if !value.is_finite() || value < 0.0 {
        return Err(Error("annotations must be finite and nonnegative".into()));
    }
    Ok(value)
}
fn range(value: &str) -> Result<[f64; 2]> {
    let value = value
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or_else(|| Error("expected an annotation range".into()))?;
    let (a, b) = value
        .split_once(',')
        .ok_or_else(|| Error("expected two range endpoints".into()))?;
    let values = [number(a.trim())?, number(b.trim())?];
    if values[0] > values[1] {
        return Err(Error("reversed annotation range".into()));
    }
    Ok(values)
}
