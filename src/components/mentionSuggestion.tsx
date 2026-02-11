/**
 * Tiptap Mention suggestion configuration.
 *
 * Uses a MutableRefObject so the items callback always reads the latest
 * person list, even though useEditor captures the config only once on mount.
 */
import { ReactRenderer } from '@tiptap/react';
import type { SuggestionOptions, SuggestionProps, SuggestionKeyDownProps } from '@tiptap/suggestion';
import { MentionList, type MentionItem, type MentionListRef } from './MentionList';
import { createSuggestionPopup, type PopupHandle } from './popupPosition';
import type { MutableRefObject } from 'react';

export type { MentionItem };

/**
 * Build a suggestion config for the Mention extension.
 *
 * @param itemsRef – a React ref that always holds the latest mentionable list
 */
export function createMentionSuggestion(
  itemsRef: MutableRefObject<MentionItem[]>,
): Omit<SuggestionOptions<MentionItem>, 'editor'> {
  return {
    items: ({ query }: { query: string }) => {
      const q = query.toLowerCase();
      return itemsRef.current
        .filter((item) => item.label.toLowerCase().includes(q))
        .slice(0, 8);
    },

    render: () => {
      let component: ReactRenderer<MentionListRef> | null = null;
      let popupHandle: PopupHandle | null = null;

      return {
        onStart: (props: SuggestionProps<MentionItem>) => {
          component = new ReactRenderer(MentionList, {
            props,
            editor: props.editor,
          });

          popupHandle = createSuggestionPopup(props.editor, props.range.to);
          popupHandle.element.appendChild(component.element);
        },

        onUpdate: (props: SuggestionProps<MentionItem>) => {
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
