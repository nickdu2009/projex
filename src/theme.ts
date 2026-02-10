import { createTheme, type MantineColorsTuple } from '@mantine/core';

// 自定义渐变友好的主色调
const indigo: MantineColorsTuple = [
  '#eef2ff',
  '#e0e7ff',
  '#c7d2fe',
  '#a5b4fc',
  '#818cf8',
  '#6366f1', // 主色
  '#4f46e5',
  '#4338ca',
  '#3730a3',
  '#312e81',
];

export const theme = createTheme({
  primaryColor: 'indigo',
  colors: {
    indigo,
  },

  // 圆角设置
  defaultRadius: 'md',
  radius: {
    xs: '4px',
    sm: '6px',
    md: '8px',
    lg: '12px',
    xl: '16px',
  },

  // 字体栈：现代无衬线字体
  fontFamily:
    'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
  fontFamilyMonospace: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',

  // 柔和阴影
  shadows: {
    xs: '0 1px 3px rgba(0, 0, 0, 0.05)',
    sm: '0 2px 8px rgba(0, 0, 0, 0.06)',
    md: '0 4px 12px rgba(0, 0, 0, 0.08)',
    lg: '0 8px 24px rgba(0, 0, 0, 0.10)',
    xl: '0 16px 48px rgba(0, 0, 0, 0.12)',
  },

  // 组件默认属性覆盖
  components: {
    Paper: {
      defaultProps: {
        radius: 'lg',
        shadow: 'sm',
        p: 'md',
      },
      styles: {
        root: {
          border: '1px solid rgba(0, 0, 0, 0.05)',
          transition: 'box-shadow 150ms ease, transform 150ms ease',
          '&:hover': {
            boxShadow: '0 4px 16px rgba(0, 0, 0, 0.1)',
          },
        },
      },
    },

    Button: {
      defaultProps: {
        radius: 'md',
      },
      styles: {
        root: {
          fontWeight: 500,
          transition: 'all 150ms ease',
        },
      },
    },

    TextInput: {
      defaultProps: {
        radius: 'md',
      },
    },

    NumberInput: {
      defaultProps: {
        radius: 'md',
      },
    },

    Select: {
      defaultProps: {
        radius: 'md',
      },
    },

    Textarea: {
      defaultProps: {
        radius: 'md',
      },
    },

    Table: {
      defaultProps: {
        striped: 'even',
        highlightOnHover: true,
      },
      styles: {
        table: {
          borderRadius: '8px',
          overflow: 'hidden',
        },
      },
    },

    Badge: {
      defaultProps: {
        radius: 'xl',
        variant: 'light',
      },
    },

    NavLink: {
      defaultProps: {
        radius: 'md',
      },
      styles: {
        root: {
          transition: 'background-color 150ms ease',
        },
      },
    },

    Modal: {
      defaultProps: {
        radius: 'lg',
      },
    },
  },
});
