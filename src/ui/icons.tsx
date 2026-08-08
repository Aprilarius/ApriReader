export type IconName =
  | "library"
  | "reading"
  | "collections"
  | "authors"
  | "series"
  | "favorite"
  | "achievement"
  | "statistics"
  | "settings"
  | "search"
  | "book"
  | "plus"
  | "folder"
  | "refresh"
  | "bookmark"
  | "notes"
  | "audio"
  | "play"
  | "pause"
  | "previous"
  | "next"
  | "rewind"
  | "forward"
  | "volume";

const paths: Record<IconName, React.ReactNode> = {
  library: (
    <path d="M4 5.5h5v13H4zM10 5.5h5v13h-5zM16.4 6.2l3.6-.9 2.7 12.7-3.6.8z" />
  ),
  reading: (
    <path d="M5 4.5h5.5c1 0 1.5.7 1.5 1.5v14c0-.9-.7-1.5-1.5-1.5H5V4.5Zm14 0h-5.5c-1 0-1.5.7-1.5 1.5v14c0-.9.7-1.5 1.5-1.5H19V4.5Z" />
  ),
  collections: <path d="M4 7h16v12H4zM7 4h10M7 10h7" />,
  authors: (
    <>
      <circle cx="9" cy="8" r="3" />
      <path d="M3.5 19c.6-4 2.4-6 5.5-6s5 2 5.5 6M15.5 7.5h5M17 11h3.5M17 15h3.5" />
    </>
  ),
  series: <path d="M6 4h12v4H6zM5 10h14v4H5zM4 16h16v4H4z" />,
  favorite: (
    <path d="m12 19-1.2-1C5.7 13.5 3 11 3 7.9 3 5.5 4.9 4 7.2 4c1.3 0 2.6.6 3.4 1.6L12 7.2l1.4-1.6A4.5 4.5 0 0 1 16.8 4C19.1 4 21 5.5 21 8c0 3-2.7 5.5-7.8 10L12 19Z" />
  ),
  achievement: (
    <path d="M8 3h8v3a4 4 0 0 1-8 0V3ZM8 5H4v2c0 2.2 1.8 4 4 4M16 5h4v2c0 2.2-1.8 4-4 4M12 10v5M8 20h8M9 15h6v5H9z" />
  ),
  statistics: <path d="M5 20v-7h4v7M10 20V4h4v16M15 20v-11h4v11M3 20h18" />,
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19 14.5 21 16l-2 3-2.3-1a8 8 0 0 1-2.2 1.3L14 22h-4l-.5-2.7A8 8 0 0 1 7.3 18L5 19l-2-3 2-1.5a8 8 0 0 1 0-5L3 8l2-3 2.3 1a8 8 0 0 1 2.2-1.3L10 2h4l.5 2.7A8 8 0 0 1 16.7 6L19 5l2 3-2 1.5a8 8 0 0 1 0 5Z" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="6.5" />
      <path d="m16 16 4 4" />
    </>
  ),
  book: <path d="M5 4h11a3 3 0 0 1 3 3v13H7a2 2 0 0 1-2-2V4Zm2 12h12" />,
  plus: <path d="M12 5v14M5 12h14" />,
  folder: <path d="M3 6h7l2 2h9v11H3z" />,
  refresh: (
    <path d="M20 7v5h-5M4 17v-5h5M6.1 8.2A7 7 0 0 1 18.5 7M17.9 15.8A7 7 0 0 1 5.5 17" />
  ),
  bookmark: <path d="M6 3h12v18l-6-4-6 4V3Z" />,
  notes: <path d="M5 3h14v18H5zM8 8h8M8 12h8M8 16h5" />,
  audio: (
    <>
      <path d="M4 13v-2a8 8 0 0 1 16 0v2" />
      <path d="M4 12h3v7H5a1 1 0 0 1-1-1v-6Zm16 0h-3v7h2a1 1 0 0 0 1-1v-6Z" />
    </>
  ),
  play: <path d="m8 5 11 7-11 7V5Z" />,
  pause: <path d="M7 5h4v14H7zM13 5h4v14h-4z" />,
  previous: <path d="M6 5v14M18 6 9 12l9 6V6Z" />,
  next: <path d="M18 5v14M6 6l9 6-9 6V6Z" />,
  rewind: <path d="M11 6 4 12l7 6V6Zm9 0-7 6 7 6V6Z" />,
  forward: <path d="m4 6 7 6-7 6V6Zm9 0 7 6-7 6V6Z" />,
  volume: (
    <>
      <path d="M4 10h4l5-4v12l-5-4H4v-4Z" />
      <path d="M16 9c1.6 1.6 1.6 4.4 0 6M18.5 6.5c3 3 3 8 0 11" />
    </>
  ),
};

export function Icon({ name }: { name: IconName }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      {paths[name]}
    </svg>
  );
}
