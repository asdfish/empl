use {
    empl::config::cli::flag::{Arguments, ArgumentsError, Flag},
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Stage {
    Lex,
    Parse,
    #[default]
    Eval,
}
impl<'a> TryFrom<&'a str> for Stage {
    type Error = UnknownStageError<'a>;

    fn try_from(stage: &'a str) -> Result<Self, Self::Error> {
        match stage {
            "lex" => Ok(Self::Lex),
            "parse" => Ok(Self::Parse),
            "eval" => Ok(Self::Eval),
            stage => Err(UnknownStageError(stage)),
        }
    }
}
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct UnknownStageError<'a>(&'a str);
impl Display for UnknownStageError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "`{}` is not a recognized stage", self.0)
    }
}
impl Error for UnknownStageError<'_> {}

#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    pub stage: Stage,
}
impl Config {
    pub fn new<'a, I, E>(mut flags: Arguments<'a, I, E>) -> Result<Self, ConfigError<'a, E>>
    where
        I: Iterator<Item = Result<&'a str, E>>,
    {
        macro_rules! value {
            ($flag:expr) => {
                flags
                    .value()
                    .map(|result| {
                        result
                            .map_err(ArgumentsError::Source)
                            .map_err(ConfigError::Arguments)
                    })
                    .unwrap_or_else(|| Err(ConfigError::NoValue($flag)))?
            };
        }

        let mut config = Self::default();

        while let Some(flag) = flags.next().transpose()? {
            match flag {
                Flag::Short('s') | Flag::Long("stage") => {
                    config.stage = Stage::try_from(value!(flag))?;
                }
                flag => return Err(ConfigError::UnknownFlag(flag)),
            }
        }

        Ok(config)
    }
}

#[derive(Clone, Debug)]
pub enum ConfigError<'a, E> {
    Arguments(ArgumentsError<'a, E>),
    NoValue(Flag<'a>),
    UnknownFlag(Flag<'a>),
    UnknownStage(UnknownStageError<'a>),
}
impl<E> Display for ConfigError<'_, E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Arguments(e) => e.fmt(f),
            Self::NoValue(flag) => write!(f, "flag `{flag}` requires an argument"),
            Self::UnknownFlag(flag) => write!(f, "unknown flag `{flag}`"),
            Self::UnknownStage(e) => e.fmt(f),
        }
    }
}
impl<E> Error for ConfigError<'_, E> where E: fmt::Debug + Display {}
impl<'a, E> From<ArgumentsError<'a, E>> for ConfigError<'a, E> {
    fn from(err: ArgumentsError<'a, E>) -> Self {
        Self::Arguments(err)
    }
}
impl<'a, E> From<UnknownStageError<'a>> for ConfigError<'a, E> {
    fn from(err: UnknownStageError<'a>) -> Self {
        Self::UnknownStage(err)
    }
}
