"use client";

import React, { useState } from "react";

type InputProps = {
  label?: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  type?: "text" | "password" | "email" | "number";
  disabled?: boolean;
};

export default function Input({ label, placeholder, value, onChange, type = "text", disabled = false }: InputProps) {
  const [focused, setFocused] = useState(false);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
      {label && (
        <label style={{ fontSize: "14px", fontWeight: 500, color: "#374151" }}>
          {label}
        </label>
      )}
      <input
        type={type}
        value={value}
        placeholder={placeholder}
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
          cursor: disabled ? "not-allowed" : "text",
          transition: "border-color 0.15s ease",
          width: "100%",
          boxSizing: "border-box",
        }}
      />
    </div>
  );
}
