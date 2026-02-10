import { create } from 'zustand';
import { peopleApi, type PersonDto } from '../api/people';

interface PersonStore {
  items: PersonDto[];
  loading: boolean;
  loaded: boolean;
  fetch: (onlyActive?: boolean) => Promise<void>;
  /** Convenience: active persons as select options */
  activeOptions: () => { value: string; label: string }[];
  invalidate: () => void;
}

export const usePersonStore = create<PersonStore>((set, get) => ({
  items: [],
  loading: false,
  loaded: false,

  fetch: async (onlyActive = true) => {
    set({ loading: true });
    try {
      const items = await peopleApi.list(onlyActive);
      set({ items, loaded: true });
    } finally {
      set({ loading: false });
    }
  },

  activeOptions: () =>
    get()
      .items.filter((p) => p.is_active)
      .map((p) => ({ value: p.id, label: p.display_name })),

  invalidate: () => set({ loaded: false }),
}));
