import { useNavigate } from "@gluxe/router";

import { Menu } from "../components/Menu";

/** Menu screen: picks the mode and hands the config to /game via location state. */
export default function MenuPage() {
  const navigate = useNavigate();
  return <Menu onStart={(config) => navigate("/game", { state: config })} />;
}
