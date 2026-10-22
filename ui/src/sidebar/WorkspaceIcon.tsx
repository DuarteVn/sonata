import {
  Anchor, Bot, Brain, Code, Compass, Cpu, Database, Feather, FlaskConical, Flame, Gem, Globe,
  Hammer, Layers, Leaf, Music, Package, Puzzle, Rocket, Shield, Star, Terminal, Wrench, Zap,
  type LucideIcon,
} from "lucide-react";

// Ids match sonata-core's workspace::palette::ICONS.
const ICONS: Record<string, LucideIcon> = {
  terminal: Terminal, code: Code, rocket: Rocket, "flask-conical": FlaskConical, bot: Bot,
  brain: Brain, cpu: Cpu, database: Database, globe: Globe, layers: Layers, package: Package,
  puzzle: Puzzle, wrench: Wrench, hammer: Hammer, leaf: Leaf, flame: Flame, zap: Zap, star: Star,
  music: Music, compass: Compass, anchor: Anchor, feather: Feather, gem: Gem, shield: Shield,
};

export function WorkspaceIcon({ icon, color, size = 16 }: { icon: string; color: string; size?: number }) {
  const Icon = ICONS[icon] ?? Terminal;
  return (
    <span className="ws-icon" data-icon={icon} data-color={color} style={{ background: color }}>
      <Icon size={size - 4} color="white" strokeWidth={2.25} aria-hidden />
    </span>
  );
}
