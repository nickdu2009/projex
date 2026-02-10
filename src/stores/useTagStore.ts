import { create } from 'zustand';
import { projectApi } from '../api/projects';

interface TagStore {
  tags: string[];
  loading: boolean;
  loaded: boolean;
  /** Fetch all unique tags from all projects (first page, large limit) */
  fetch: () => Promise<void>;
  invalidate: () => void;
}

export const useTagStore = create<TagStore>((set) => ({
  tags: [],
  loading: false,
  loaded: false,

  fetch: async () => {
    set({ loading: true });
    try {
      // Fetch all projects (unarchived + archived) to collect all tags
      const result = await projectApi.list({
        onlyUnarchived: false,
        limit: 200,
      });
      const tagSet = new Set<string>();
      for (const item of result.items) {
        for (const tag of item.tags ?? []) {
          tagSet.add(tag);
        }
      }
      set({ tags: Array.from(tagSet).sort(), loaded: true });
    } finally {
      set({ loading: false });
    }
  },

  invalidate: () => set({ loaded: false }),
}));
