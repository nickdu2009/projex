import { create } from 'zustand';
import { partnersApi, type PartnerDto } from '../api/partners';

interface PartnerStore {
  items: PartnerDto[];
  loading: boolean;
  /** Fetched at least once */
  loaded: boolean;
  fetch: (onlyActive?: boolean) => Promise<void>;
  /** Convenience: active partners as select options */
  activeOptions: () => { value: string; label: string }[];
  invalidate: () => void;
}

export const usePartnerStore = create<PartnerStore>((set, get) => ({
  items: [],
  loading: false,
  loaded: false,

  fetch: async (onlyActive = true) => {
    set({ loading: true });
    try {
      const items = await partnersApi.list(onlyActive);
      set({ items, loaded: true });
    } finally {
      set({ loading: false });
    }
  },

  activeOptions: () =>
    get()
      .items.filter((p) => p.is_active)
      .map((p) => ({ value: p.id, label: p.name })),

  invalidate: () => set({ loaded: false }),
}));
