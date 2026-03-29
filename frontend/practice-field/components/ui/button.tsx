"use client";

import React from "react";

type ButtonProps = {
  text: string;
  color: string; // base color (hex)
  onPress: () => void;
};

function darkenHex(hex: string, amount: number) {
  let col = hex.replace("#", "");

  if (col.length === 3) {
    col = col.split("").map(c => c + c).join("");
  }

  const num = parseInt(col, 16);

  let r = (num >> 16) - amount;
  let g = ((num >> 8) & 0x00ff) - amount;
  let b = (num & 0x0000ff) - amount;

  r = Math.max(0, r);
  g = Math.max(0, g);
  b = Math.max(0, b);

  return `#${(r << 16 | g << 8 | b).toString(16).padStart(6, "0")}`;
}

export default function Button({ text, color, onPress }: ButtonProps) {
  const darker = darkenHex(color, 30);

  return (
    <button
      onClick={onPress}
      style={{
        backgroundColor: color,
      }}
      onMouseDown={(e) => {
        (e.currentTarget as HTMLButtonElement).style.backgroundColor = darker;
      }}
      onMouseUp={(e) => {
        (e.currentTarget as HTMLButtonElement).style.backgroundColor = color;
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLButtonElement).style.backgroundColor = color;
      }}
      className="px-4 py-2 rounded text-white transition"
    >
      {text}
    </button>
  );
}