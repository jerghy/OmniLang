/** Style-command interfaces.
 *  Style commands operate on the style registry store directly and accept a
 *  CommandContext parameter that is unused (kept for interface compatibility). */

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface CommandContext {}

export interface Command {
  readonly type: string;
  readonly description: string;
  execute(ctx: CommandContext): void;
  undo(ctx: CommandContext): void;
}
