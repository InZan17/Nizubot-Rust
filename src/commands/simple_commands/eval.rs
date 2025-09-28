use evalexpr::{context_map, EvalexprError, Value};
use poise::CreateReply;

use crate::{Context, Error};

/// I will evaluate a math expression!
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn eval(
    ctx: Context<'_>,
    #[max_length = 200]
    #[description = "What's the math expression you want me to solve?"]
    expression: String,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);

    let mut context = context_map! {
        "sin" => Function::new(|argument| {
            if let Ok(int) = argument.as_int() {
                Ok(Value::Float((int as f64).sin()))
            } else if let Ok(float) = argument.as_float() {
                Ok(Value::Float(float.sin()))
            } else {
                Err(EvalexprError::expected_number(argument.clone()))
            }
        }),
        "cos" => Function::new(|argument| {
            if let Ok(int) = argument.as_int() {
                Ok(Value::Float((int as f64).cos()))
            } else if let Ok(float) = argument.as_float() {
                Ok(Value::Float(float.cos()))
            } else {
                Err(EvalexprError::expected_number(argument.clone()))
            }
        })
    }?;

    let value = evalexpr::eval_with_context_mut(&expression, &mut context)?;

    ctx.send(
        CreateReply::default()
            .content(value.to_string())
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}
