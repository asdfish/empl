// use {
//     crate::{
//         config::KeyAction,
//         display::{
//             damage::DamageList,
//             state::DisplayState,
//         },
//         select::select2,
//         tasks::display::{
//             DisplayTask,
//             EventTask,
//         },
//     },
//     std::{
//         error::Error,
//         fmt::{self, Display, Formatter},
//         io,
//     },
//     tokio::sync::mpsc,
// };

// #[derive(Debug)]
// pub struct State<'a> {
//     action_rx: mpsc::UnboundedReceiver<KeyAction>,
//     display_tx: mpsc::UnboundedSender<DamageList>,
//     state: DisplayState<'a>,

//     display_task: DisplayTask,
//     event_task: EventTask,
// }
// impl<'a> State<'a> {
//     pub async fn run(mut self) -> Result<(), StateError> {
//         loop {
//             match select2(self.display_task.run().pipe(StateError::Render), self.event_task.run().pipe(StateError::ActionSend)) {
//                 Ok(()) => break Ok(()),
//                 Err(StateError::ActionSend(mpsc::error::SendError<KeyAction>(action))) => {
//                     let (action_tx, action_rx) = mpsc::unbounded_channel();
//                     self.action_rx = action_rx;
//                     action_tx.send();
//                     event_task.action_tx = action_tx;
//                 },
//                 Err(e @ StateError::Render(_)) => return Err(e),
//             }
//         }
//     }
// }

// #[derive(Clone, Debug)]
// pub enum StateError {
//     ActionSend(mpsc::error::SendError<KeyAction>),
//     Render(io::Error),
// }
// impl Display for StateError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
//         match self {
//             Self::ActionSend(e) => write!(f, "failed to send key action: {e}"),
//             Self::Render(e) => write!(f, "failure during rendering: {e}"),
//         }
//     }
// }
// impl Error for StateError {}
