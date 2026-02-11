/**
 * Tiptap Slash Command extension.
 *
 * Triggered by typing "/" at the start of a new line or after a space.
 * Uses the same suggestion mechanism as @mention but with editor commands.
 */
import { Extension } from '@tiptap/core';
import { ReactRenderer } from '@tiptap/react';
import Suggestion from '@tiptap/suggestion';
import type { SuggestionOptions, SuggestionProps, SuggestionKeyDownProps } from '@tiptap/suggestion';
import type { Editor, Range } from '@tiptap/core';
import { SlashCommandList, type SlashCommandItem, type SlashCommandListRef } from './SlashCommandList';
import { createSuggestionPopup, type PopupHandle } from './popupPosition';
import {
  IconH1,
  IconH2,
  IconH3,
  IconList,
  IconListNumbers,
  IconListCheck,
  IconQuote,
  IconCode,
  IconMinus,
  IconTable,
  IconPhoto,
  IconCalendar,
} from '@tabler/icons-react';

export type { SlashCommandItem };

/**
 * Each slash command item stores an `actionId` instead of an `action()` closure,
 * because the `items` callback from tiptap/suggestion doesn't receive `range`.
 * The actual editor command is executed in the `command` callback which has both editor & range.
 */
export interface SlashCommandDef {
  id: string;
  label: string;
  description: string;
  icon: SlashCommandItem['icon'];
}

/** Static list of available slash commands */
export function getSlashCommandDefs(t: (key: string) => string): SlashCommandDef[] {
  return [
    { id: 'heading1', label: t('slash.heading1'), description: t('slash.heading1Desc'), icon: IconH1 },
    { id: 'heading2', label: t('slash.heading2'), description: t('slash.heading2Desc'), icon: IconH2 },
    { id: 'heading3', label: t('slash.heading3'), description: t('slash.heading3Desc'), icon: IconH3 },
    { id: 'bulletList', label: t('slash.bulletList'), description: t('slash.bulletListDesc'), icon: IconList },
    { id: 'orderedList', label: t('slash.orderedList'), description: t('slash.orderedListDesc'), icon: IconListNumbers },
    { id: 'taskList', label: t('slash.taskList'), description: t('slash.taskListDesc'), icon: IconListCheck },
    { id: 'blockquote', label: t('slash.blockquote'), description: t('slash.blockquoteDesc'), icon: IconQuote },
    { id: 'codeBlock', label: t('slash.codeBlock'), description: t('slash.codeBlockDesc'), icon: IconCode },
    { id: 'divider', label: t('slash.divider'), description: t('slash.dividerDesc'), icon: IconMinus },
    { id: 'table', label: t('slash.table'), description: t('slash.tableDesc'), icon: IconTable },
    { id: 'image', label: t('slash.image'), description: t('slash.imageDesc'), icon: IconPhoto },
    { id: 'date', label: t('slash.date'), description: t('slash.dateDesc'), icon: IconCalendar },
  ];
}

/** Execute the slash command by id – called from the `command` callback which provides editor + range */
function executeSlashCommand(id: string, editor: Editor, range: Range) {
  switch (id) {
    case 'heading1':
      editor.chain().focus().deleteRange(range).setNode('heading', { level: 1 }).run();
      break;
    case 'heading2':
      editor.chain().focus().deleteRange(range).setNode('heading', { level: 2 }).run();
      break;
    case 'heading3':
      editor.chain().focus().deleteRange(range).setNode('heading', { level: 3 }).run();
      break;
    case 'bulletList':
      editor.chain().focus().deleteRange(range).toggleBulletList().run();
      break;
    case 'orderedList':
      editor.chain().focus().deleteRange(range).toggleOrderedList().run();
      break;
    case 'taskList':
      editor.chain().focus().deleteRange(range).toggleTaskList().run();
      break;
    case 'blockquote':
      editor.chain().focus().deleteRange(range).toggleBlockquote().run();
      break;
    case 'codeBlock':
      editor.chain().focus().deleteRange(range).toggleCodeBlock().run();
      break;
    case 'divider':
      editor.chain().focus().deleteRange(range).setHorizontalRule().run();
      break;
    case 'table':
      editor.chain().focus().deleteRange(range).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run();
      break;
    case 'image': {
      editor.chain().focus().deleteRange(range).run();
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = 'image/png,image/jpeg,image/webp,image/gif';
      input.onchange = () => {
        const file = input.files?.[0];
        if (!file) return;
        const reader = new FileReader();
        reader.onload = () => {
          editor.chain().focus().setImage({ src: reader.result as string }).run();
        };
        reader.readAsDataURL(file);
      };
      input.click();
      break;
    }
    case 'date': {
      const today = new Date();
      const formatted = today.toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
      editor.chain().focus().deleteRange(range).insertContent(formatted).run();
      break;
    }
  }
}

/** The Tiptap extension that wires "/" suggestion */
export const SlashCommand = Extension.create({
  name: 'slashCommand',

  addOptions() {
    return {
      suggestion: {
        char: '/',
        startOfLine: false,
      } as Partial<SuggestionOptions<SlashCommandItem>>,
    };
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        ...this.options.suggestion,
      }),
    ];
  },
});

/** Build suggestion config (render the popup) */
export function createSlashSuggestion(
  tFn: () => (key: string) => string,
): Partial<SuggestionOptions<SlashCommandItem>> {
  return {
    char: '/',
    startOfLine: false,

    items: ({ query }: { query: string }) => {
      const t = tFn();
      const defs = getSlashCommandDefs(t);
      // Convert defs to SlashCommandItem with a no-op action (real action in `command`)
      const items: SlashCommandItem[] = defs.map((d) => ({
        ...d,
        action: () => {},
      }));
      const q = query.toLowerCase();
      if (!q) return items;
      return items.filter(
        (cmd) =>
          cmd.label.toLowerCase().includes(q) ||
          cmd.id.toLowerCase().includes(q),
      );
    },

    command: ({ editor, range, props }: { editor: Editor; range: Range; props: SlashCommandItem }) => {
      executeSlashCommand(props.id, editor, range);
    },

    render: () => {
      let component: ReactRenderer<SlashCommandListRef> | null = null;
      let popupHandle: PopupHandle | null = null;

      return {
        onStart: (props: SuggestionProps<SlashCommandItem>) => {
          component = new ReactRenderer(SlashCommandList, {
            props,
            editor: props.editor,
          });

          popupHandle = createSuggestionPopup(props.editor, props.range.to);
          popupHandle.element.appendChild(component.element);
        },

        onUpdate: (props: SuggestionProps<SlashCommandItem>) => {
          component?.updateProps(props);
          popupHandle?.setAnchorPos(props.range.to);
          popupHandle?.reposition();
        },

        onKeyDown: (props: SuggestionKeyDownProps) => {
          if (props.event.key === 'Escape') {
            popupHandle?.destroy();
            popupHandle = null;
            component?.destroy();
            component = null;
            return true;
          }
          return component?.ref?.onKeyDown(props) ?? false;
        },

        onExit: () => {
          popupHandle?.destroy();
          popupHandle = null;
          component?.destroy();
          component = null;
        },
      };
    },
  };
}

