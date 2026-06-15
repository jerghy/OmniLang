/** 将多个 Command 组合为一个原子操作（单一撤销/重做入口）。
 *  execute 按正向顺序执行，undo 按反向顺序撤销。 */

import type { Command, CommandContext } from "./types";

export class CompositeCommand implements Command {
  readonly type = "CompositeCommand";
  readonly description: string;
  private commands: readonly Command[];

  constructor(commands: Command[], description = "批量操作") {
    this.commands = commands;
    this.description = description;
  }

  execute(ctx: CommandContext): void {
    for (const cmd of this.commands) {
      cmd.execute(ctx);
    }
  }

  undo(ctx: CommandContext): void {
    // Reverse order for correct undo
    for (let i = this.commands.length - 1; i >= 0; i--) {
      this.commands[i].undo(ctx);
    }
  }
}
