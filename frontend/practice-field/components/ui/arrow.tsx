"use client";

type ArrowProps = {
  direction?: "left" | "right" | "up" | "down";
  size?: number; // in px
  color?: string;
  className?: string;
};

export default function Arrow({
  direction = "right",
  size = 24,
  color = "currentColor",
  className = "",
}: ArrowProps) {
  const rotations = {
    right: "rotate(0deg)",
    down: "rotate(90deg)",
    left: "rotate(180deg)",
    up: "rotate(270deg)",
  };

  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth="6"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{
        width: `${size}px`,
        height: `${size}px`,
        transform: rotations[direction],
      }}
      className={className}
    >
      {/* base arrow pointing right */}
      <path d="M5 12h14M13 5l7 7-7 7" />
    </svg>
  );
}