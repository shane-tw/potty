use std::fmt;
use std::str::FromStr;
use std::io::{Read, Write, BufRead, BufReader};
use snailquote::{unescape};
use regex::Regex;
use std::mem;

pub struct Pot {
    pub messages: Vec<PotMessage>,
}

pub struct PotMessage {
    pub comments: Vec<PotComment>,
    pub context: Option<String>,
    pub id: Option<String>,
    pub id_plural: Option<String>,
    pub strings: Vec<String>,
}

pub struct PotComment {
    pub kind: PotCommentKind,
    pub content: String,
}

pub enum PotCommentKind {
    Reference,
    Extracted,
    Flag,
    Previous,
    Translator,
}

struct PotCommand {
    key: String,
    value: String,
    index: Option<usize>
}

impl Default for PotMessage {
    fn default() -> Self {
        PotMessage {
            comments: Vec::new(),
            context: None,
            id: None,
            id_plural: None,
            strings: Vec::new(),
        }
    }
}

impl fmt::Display for PotMessage {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for comment in &self.comments {
            writeln!(f, "{}", comment)?;
        }
        if let Some(ref ctx) = self.context {
            writeln!(f, "msgctxt \"{}\"", ctx)?;
        }
        if let Some(ref id) = self.id {
            writeln!(f, "msgid \"{}\"", id)?;
        }
        if let Some(ref id_plural) = self.id_plural {
            writeln!(f, "msgid_plural \"{}\"", id_plural)?;
        }
        for (i, string) in self.strings.iter().enumerate() {
            if self.id_plural.is_some() {
                writeln!(f, "msgstr[{}] \"{}\"", i, string)?;
            } else {
                writeln!(f, "msgstr \"{}\"", string)?;
            }
        }
        Ok(())
	}
}

impl PotMessage {
    pub fn new() -> Self {
        Default::default()
    }

    fn is_valid(&self) -> bool {
        self.id.is_some() && (self.strings.len() == 1 || (self.id_plural.is_some() && self.strings.len() > 1))
    }
}

impl fmt::Display for PotComment {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{} {}", self.kind, self.content)
	}
}

impl FromStr for PotComment {
    type Err = ();
    fn from_str(s: &str) -> Result<PotComment, Self::Err> {
        if !PotComment::is_comment(s) {
            return Err(());
        }
        let comment_type = PotCommentKind::from_str(s).unwrap();
        let content = match comment_type {
            PotCommentKind::Translator => &s[1..],
            _ => &s[2..],
        };
        Ok(PotComment{
            content: content.trim_start().to_string(),
            kind: comment_type
        })
    }
}

impl PotComment {
    pub fn is_comment(s: &str) -> bool {
        !s.is_empty() && &s[0..1] == "#"
    }
}

impl fmt::Display for PotCommentKind {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match &self {
            PotCommentKind::Reference => ":",
            PotCommentKind::Extracted => ".",
            PotCommentKind::Flag => ",",
            PotCommentKind::Previous => "|",
            _ => "",
		})
	}
}

impl FromStr for PotCommentKind {
    type Err = ();
    fn from_str(s: &str) -> Result<PotCommentKind, Self::Err> {
        if !PotComment::is_comment(s) {
            return Err(())
        }
        Ok(match &s[1..2] {
            ":" => PotCommentKind::Reference,
            "." => PotCommentKind::Extracted,
            "," => PotCommentKind::Flag,
            "|" => PotCommentKind::Previous,
            _ => PotCommentKind::Translator,
		})
    }
}

impl Default for Pot {
    fn default() -> Self {
        Pot {
            messages: Vec::new()
        }
    }
}

impl Pot {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn read<R: Read>(reader: &mut R) -> Pot {
        let f = BufReader::new(reader);
        let mut pot = Pot::new();
        let mut message = PotMessage::new();
        let mut command = PotCommand::new();

        let re = Regex::new(r#"^"(.*?[^\\])?"$"#).unwrap();

        for line in f.lines() {
            let s = line.unwrap();
            if let Ok(comment) = s.parse::<PotComment>() {
                if message.is_valid() {
                    pot.messages.push(message);
                    message = PotMessage::new();
                }
                message.comments.push(comment);
            } else if let Ok(cmd) = s.parse::<PotCommand>() {
                if !cmd.can_apply(&mut message) {
                    pot.messages.push(message);
                    message = PotMessage::new();
                }
                cmd.apply(&mut message);
                command = cmd;
            } else if let Some(caps) = re.captures(&s) {
                let s_msg = caps.get(1).and_then(|m| Some(m.as_str())).unwrap_or_default();
                command.value.push_str(unescape(s_msg).unwrap().as_ref());
                command.force_apply(&mut message);
            }
        }

        if pot.messages.is_empty() || pot.messages.last().unwrap().id.as_ref() != message.id.as_ref() {
            pot.messages.push(message);
        }

        pot
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        for (i, message) in self.messages.iter().enumerate() {
            writer.write(format!("{}", message).as_ref())?;
            if i < self.messages.len() - 1 {
                writer.write("\n".as_ref())?;
            }
        }
        Ok(())
    }
}

impl FromStr for PotCommand {
    type Err = ();
    fn from_str(s: &str) -> Result<PotCommand, Self::Err> {
        let re = Regex::new(r#"^(?P<cmd>[a-z_]+)(?:\[(?P<idx>[0-9]+)\])? "(?P<val>.*?[^\\])?""#).unwrap();
        if let Some(caps) = re.captures(s) {
            let cmd = caps.name("cmd").and_then(|m| Some(m.as_str())).unwrap_or_default();
            let idx = caps.name("idx").and_then(|m| Some(m.as_str())).unwrap_or_default();
            let val = caps.name("val").and_then(|m| Some(m.as_str())).unwrap_or_default();

            let mut cmd = PotCommand{
                key: cmd.to_string(),
                index: None,
                value: unescape(val).unwrap(),
            };

            if !idx.is_empty() {
                cmd.index = Some(idx.parse::<usize>().unwrap());
            }

            return Ok(cmd);
        }
        Err(())
    }
}

impl Default for PotCommand {
    fn default() -> Self {
        PotCommand {
            key: String::new(),
            value: String::new(),
            index: None
        }
    }
}

impl PotCommand {
    pub fn new() -> Self {
        Default::default()
    }

    fn can_apply(&self, msg: &PotMessage) -> bool {
        match self.key.as_str() {
            "msgctxt" => msg.context.is_none() && msg.id.is_none() && msg.id_plural.is_none() && msg.strings.is_empty(),
            "msgid" => msg.id.is_none() && msg.id_plural.is_none() && msg.strings.is_empty(),
            "msgid_plural" => msg.id_plural.is_none() && msg.strings.is_empty(),
            "msgstr" => {
                let idx = self.index.unwrap_or_default();
                return idx + 1 > msg.strings.len();
            },
            _ => false,
        }
    }

    fn force_apply(&self, msg: &mut PotMessage) {
        let val = self.value.clone();
        match self.key.as_str() {
            "msgctxt" => { msg.context = Some(val) },
            "msgid" => { msg.id = Some(val) },
            "msgid_plural" => { msg.id_plural = Some(val) },
            "msgstr" => {
                let idx = self.index.unwrap_or_default();
                if idx + 1 > msg.strings.len() {
                    msg.strings.push(val);
                } else {
                    mem::replace(&mut msg.strings[idx], val);
                }
            },
            _ => (),
        }
    }

    fn apply(&self, msg: &mut PotMessage) -> bool {
        if !self.can_apply(&msg) {
            return false;
        }
        self.force_apply(msg);
        true
    }
}
