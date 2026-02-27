import { RichTextEditor as MantineRTE, Link as TiptapLink } from '@mantine/tiptap';
import { useEditor, type JSONContent } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import Underline from '@tiptap/extension-underline';
import Highlight from '@tiptap/extension-highlight';
import TextAlign from '@tiptap/extension-text-align';
import Superscript from '@tiptap/extension-superscript';
import Subscript from '@tiptap/extension-subscript';
import Image from '@tiptap/extension-image';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { Table } from '@tiptap/extension-table';
import { TableRow } from '@tiptap/extension-table-row';
import { TableCell } from '@tiptap/extension-table-cell';
import { TableHeader } from '@tiptap/extension-table-header';
import Mention from '@tiptap/extension-mention';
import { IconPhoto, IconTable } from '@tabler/icons-react';
import { ActionIcon, FileButton, Tooltip } from '@mantine/core';
import { useTranslation } from 'react-i18next';
import { useEffect, useMemo, useRef } from 'react';
import { useIsMobile } from '../utils/useIsMobile';
import { createMentionSuggestion, type MentionItem } from './mentionSuggestion';
import { SlashCommand, createSlashSuggestion } from './slashCommand';

interface RichTextEditorProps {
  content: string | JSONContent;
  onChange?: (content: JSONContent) => void;
  editable?: boolean;
  placeholder?: string;
  /** List of mentionable people – triggers with "@" */
  mentionItems?: MentionItem[];
}

export function RichTextEditor({
  content,
  onChange,
  editable = true,
  placeholder,
  mentionItems = [],
}: RichTextEditorProps) {
  const { t } = useTranslation();
  const isMobile = useIsMobile();

  // Keep a mutable ref so the suggestion callback always reads fresh data,
  // even though useEditor captures extensions only on mount.
  const mentionItemsRef = useRef<MentionItem[]>(mentionItems);
  useEffect(() => {
    mentionItemsRef.current = mentionItems;
  }, [mentionItems]);

  // Stable reference – created once, reads from ref each time.
  // The ref is passed as a container; actual .current access happens inside
  // suggestion callbacks (user typing), not during render.
  const mentionSuggestion = useMemo(
    // eslint-disable-next-line react-hooks/refs
    () => createMentionSuggestion(mentionItemsRef),
    [],
  );

  // Slash command suggestion – uses a getter so t() is always fresh.
  // Same pattern: the arrow function captures tRef but only reads .current
  // inside suggestion callbacks, not during render.
  const tRef = useRef(t);
  useEffect(() => {
    tRef.current = t;
  }, [t]);
  const slashSuggestion = useMemo(
    // eslint-disable-next-line react-hooks/refs
    () => createSlashSuggestion(() => tRef.current),
    [],
  );

  const editor = useEditor({
    extensions: [
      StarterKit,
      TiptapLink,
      Underline,
      Highlight,
      Superscript,
      Subscript,
      TextAlign.configure({ types: ['heading', 'paragraph'] }),
      Placeholder.configure({ placeholder: placeholder ?? t('comment.placeholder') }),
      Image.configure({ inline: true, allowBase64: true }),
      TaskList,
      TaskItem.configure({ nested: true }),
      Table.configure({ resizable: true }),
      TableRow,
      TableHeader,
      TableCell,
      Mention.configure({
        HTMLAttributes: { class: 'mention' },
        suggestion: mentionSuggestion,
      }),
      SlashCommand.configure({
        suggestion: slashSuggestion,
      }),
    ],
    content: typeof content === 'string' ? JSON.parse(content) : content,
    editable,
    onUpdate: ({ editor: ed }) => {
      if (onChange) {
        onChange(ed.getJSON());
      }
    },
  });

  const handleImageUpload = (file: File | null) => {
    if (!file || !editor) return;
    const reader = new FileReader();
    reader.onload = () => {
      editor.chain().focus().setImage({ src: reader.result as string }).run();
    };
    reader.readAsDataURL(file);
  };

  if (!editor) return null;

  // Read-only mode: render content only, no border/toolbar
  if (!editable) {
    return (
      <MantineRTE editor={editor}>
        <MantineRTE.Content style={{ border: 'none' }} />
      </MantineRTE>
    );
  }

  return (
    <MantineRTE editor={editor}>
      <MantineRTE.Toolbar style={{ flexWrap: 'wrap' }}>
        {/* Core formatting – always shown */}
        <MantineRTE.ControlsGroup>
          <MantineRTE.Bold />
          <MantineRTE.Italic />
          <MantineRTE.Underline />
          {!isMobile && <MantineRTE.Strikethrough />}
          {!isMobile && <MantineRTE.Highlight />}
          {!isMobile && <MantineRTE.Code />}
          {!isMobile && <MantineRTE.ClearFormatting />}
        </MantineRTE.ControlsGroup>

        {/* Headings */}
        <MantineRTE.ControlsGroup>
          <MantineRTE.H1 />
          <MantineRTE.H2 />
          {!isMobile && <MantineRTE.H3 />}
          {!isMobile && <MantineRTE.H4 />}
        </MantineRTE.ControlsGroup>

        {/* Lists & blocks */}
        <MantineRTE.ControlsGroup>
          <MantineRTE.BulletList />
          <MantineRTE.OrderedList />
          <MantineRTE.TaskList />
          {!isMobile && <MantineRTE.Blockquote />}
          {!isMobile && <MantineRTE.Hr />}
          {!isMobile && <MantineRTE.CodeBlock />}
        </MantineRTE.ControlsGroup>

        {/* Links */}
        <MantineRTE.ControlsGroup>
          <MantineRTE.Link />
          <MantineRTE.Unlink />
        </MantineRTE.ControlsGroup>

        {/* Alignment – hide on mobile to save space */}
        {!isMobile && (
          <MantineRTE.ControlsGroup>
            <MantineRTE.AlignLeft />
            <MantineRTE.AlignCenter />
            <MantineRTE.AlignRight />
            <MantineRTE.AlignJustify />
          </MantineRTE.ControlsGroup>
        )}

        {/* Insert: Image + Table */}
        <MantineRTE.ControlsGroup>
          <FileButton onChange={handleImageUpload} accept="image/png,image/jpeg,image/webp,image/gif">
            {(props) => (
              <Tooltip label={t('comment.insertImage')} withArrow>
                <ActionIcon {...props} variant="default" size={26}>
                  <IconPhoto size={16} stroke={1.5} />
                </ActionIcon>
              </Tooltip>
            )}
          </FileButton>

          {!isMobile && (
            <Tooltip label={t('comment.insertTable')} withArrow>
              <ActionIcon
                variant="default"
                size={26}
                onClick={() =>
                  editor.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()
                }
              >
                <IconTable size={16} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          )}
        </MantineRTE.ControlsGroup>

        {/* Undo/Redo */}
        <MantineRTE.ControlsGroup>
          <MantineRTE.Undo />
          <MantineRTE.Redo />
        </MantineRTE.ControlsGroup>
      </MantineRTE.Toolbar>

      <MantineRTE.Content />
    </MantineRTE>
  );
}
