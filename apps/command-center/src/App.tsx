import { useState } from "react";
import { Warehouse, ProjectSummary } from "./components/Warehouse";
import { RemixLab } from "./components/RemixLab";
import { AiomeLine } from "./components/AiomeLine";
import { JobDashboard } from "./components/JobDashboard";
import { KarmaViewer } from "./components/KarmaViewer";
import { LayoutDashboard, Library, Settings2, Clock, Database } from "lucide-react";
import { clsx } from 'clsx';

function App() {
  const [activeTab, setActiveTab] = useState<'monitor' | 'warehouse' | 'remix' | 'jobs' | 'karma'>('monitor');
  const [selectedProject, setSelectedProject] = useState<ProjectSummary | null>(null);

  const handleRemix = (project: ProjectSummary) => {
    setSelectedProject(project);
    setActiveTab('remix');
  };

  return (
    <div className="flex h-screen w-screen bg-sonar-black text-gray-200 font-sans overflow-hidden">
      {/* Sidebar Navigation */}
      <nav className="w-16 flex flex-col items-center py-6 border-r border-gray-800 bg-black/50 backdrop-blur-sm z-50">
        <div className="mb-8 p-2 bg-sonar-green/10 rounded-full">
          <div className="w-4 h-4 bg-sonar-green rounded-full shadow-[0_0_10px_#00FF41]"></div>
        </div>

        <div className="flex flex-col gap-6">
          <NavButton
            active={activeTab === 'monitor'}
            onClick={() => setActiveTab('monitor')}
            icon={<LayoutDashboard size={20} />}
          />
          <NavButton
            active={activeTab === 'jobs'}
            onClick={() => setActiveTab('jobs')}
            icon={<Clock size={20} />}
          />
          <NavButton
            active={activeTab === 'karma'}
            onClick={() => setActiveTab('karma')}
            icon={<Database size={20} />}
          />
          <NavButton
            active={activeTab === 'warehouse'}
            onClick={() => setActiveTab('warehouse')}
            icon={<Library size={20} />}
          />
          <NavButton
            active={activeTab === 'remix'}
            onClick={() => setActiveTab('remix')}
            icon={<Settings2 size={20} />}
          />
        </div>
      </nav>

      {/* Main Content Area */}
      <main className="flex-1 relative">
        <div className="absolute inset-0 bg-[url('/grid.svg')] opacity-5 pointer-events-none"></div>

        {activeTab === 'monitor' && <AiomeLine />}

        {activeTab === 'jobs' && <JobDashboard />}

        {activeTab === 'karma' && <KarmaViewer />}

        {activeTab === 'warehouse' && (
          <Warehouse onRemix={handleRemix} />
        )}

        {activeTab === 'remix' && (
          <RemixLab targetProject={selectedProject} />
        )}
      </main>
    </div>
  );
}

const NavButton = ({ active, onClick, icon }: { active: boolean, onClick: () => void, icon: React.ReactNode }) => (
  <button
    onClick={onClick}
    className={clsx(
      "p-3 rounded-lg transition-all duration-300 relative group",
      active ? "text-sonar-green bg-sonar-green/10" : "text-gray-500 hover:text-gray-300"
    )}
  >
    {icon}
    {active && (
      <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-sonar-green rounded-r shadow-[0_0_15px_#00FF41]" />
    )}
  </button>
);

export default App;
