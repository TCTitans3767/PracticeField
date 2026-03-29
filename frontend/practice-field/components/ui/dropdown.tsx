"use client";

import React, { useState } from "react";

type DropdownProps = {
  label?: string;
  options: { label: string; value: string }[];
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
};

export default function Dropdown({ label, options, value, onChange, disabled = false }: DropdownProps) {
  const [focused, setFocused] = useState(false);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
      {label && (
        <label style={{ fontSize: "14px", fontWeight: 500, color: "#374151" }}>
          {label}
        </label>
      )}
      <select
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        style={{
          padding: "8px 12px",
          borderRadius: "6px",
          border: `2px solid ${focused ? "#3b82f6" : "#d1d5db"}`,
          outline: "none",
          fontSize: "14px",
          backgroundColor: disabled ? "#f3f4f6" : "white",
          color: disabled ? "#9ca3af" : "#111827",
          cursor: disabled ? "not-allowed" : "pointer",
          transition: "border-color 0.15s ease",
          width: "100%",
          boxSizing: "border-box",
        }}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}
