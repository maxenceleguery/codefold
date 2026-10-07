/**
 * A landing page component (toy fixture).
 */

import { useState } from "react";

export interface PageProps {
  title: string;
  user?: { name: string; id: number };
}

export function Page({ title, user }: PageProps) {
  const [count, setCount] = useState(0);

  function handleClick() {
    setCount((c) => c + 1);
  }

  return (
    <div className="container">
      <h1>{title}</h1>
      {user && <p>Welcome, {user.name}</p>}
      <button onClick={handleClick}>clicked {count} times</button>
    </div>
  );
}

export const Footer = () => (
  <footer>
    <span>© 2026</span>
  </footer>
);

function internalHelper(): number {
  return 42;
}
