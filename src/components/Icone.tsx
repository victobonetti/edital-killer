// Ícones de traço fino, desenhados para o tamanho do texto ao lado (1em).
const PATHS = {
  ajustes:
    "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7.6 7.6 0 0 0-2-1.2L14.5 3h-4l-.4 2.6a7.6 7.6 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7.4 7.4 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7.6 7.6 0 0 0 2 1.2l.4 2.6h4l.4-2.6a7.6 7.6 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2Z",
  voltar: "M15 5l-7 7 7 7",
  mais: "M12 5v14M5 12h14",
  lapis: "M4 20h4L19 9l-4-4L4 16v4ZM13.5 6.5l4 4",
  documento: "M7 3h7l5 5v13H7V3Zm7 0v5h5M10 13h6M10 17h6",
  visao: "M4 5h16M4 10h10M4 15h16M4 20h8",
  seta: "M5 12h14M13 6l6 6-6 6",
  check: "M5 12.5l4.5 4.5L19 7.5",
  alerta: "M12 4l9 16H3L12 4Zm0 6v4m0 3v.5",
  lixeira: "M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13",
  chevron: "M9 6l6 6-6 6",
  pasta: "M3 6h6l2 2h10v11H3V6Z",
  fechar: "M6 6l12 12M18 6L6 18",
} as const;

export type NomeIcone = keyof typeof PATHS;

export function Icone({ nome, className }: { nome: NomeIcone; className?: string }) {
  return (
    <svg
      className={`icone ${className ?? ""}`}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d={PATHS[nome]} />
    </svg>
  );
}
