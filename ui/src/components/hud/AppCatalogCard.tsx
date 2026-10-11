import React, { useState, useEffect } from "react";
import { List, Search, Play, RefreshCw, Check } from "lucide-react";
import { ApplicationItem, DialSectorDefinition } from "../../types/config";
import { tauriApi } from "../../services/ipc";

interface Props {
  onAssignToSector?: (sectorId: number, app: ApplicationItem) => void;
  sectors?: DialSectorDefinition[];
}

export const AppCatalogCard: React.FC<Props> = ({ onAssignToSector, sectors }) => {
  const [apps, setApps] = useState<ApplicationItem[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [launchStatus, setLaunchStatus] = useState<string | null>(null);
  const [assignedAppId, setAssignedAppId] = useState<string | null>(null);

  const fetchApps = async () => {
    setLoading(true);
    try {
      const res = await tauriApi.getCatalogedApps();
      if (res?.apps) {
        setApps(res.apps);
      }
    } catch (e) {
      console.error("Failed to load cataloged apps:", e);
    } finally {
      setLoading(false);
    }
  };

  const refreshApps = async () => {
    setLoading(true);
    try {
      const res = await tauriApi.refreshCatalogedApps();
      if (res?.apps) {
        setApps(res.apps);
      }
    } catch (e) {
      console.error("Failed to refresh cataloged apps:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchApps();
  }, []);

  const handleLaunch = async (path: string, name: string) => {
    try {
      await tauriApi.launchApp(path);
      setLaunchStatus(`Launched ${name}`);
      setTimeout(() => setLaunchStatus(null), 3000);
    } catch (e: any) {
      setLaunchStatus(`Failed to launch: ${e?.message || e}`);
      setTimeout(() => setLaunchStatus(null), 3000);
    }
  };

  const handleAssign = (sectorId: number, app: ApplicationItem) => {
    if (onAssignToSector) {
      onAssignToSector(sectorId, app);
      setAssignedAppId(`${app.id}-${sectorId}`);
      setTimeout(() => setAssignedAppId(null), 2500);
    }
  };

  const filteredApps = apps.filter(
    (app) =>
      app.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      app.path.toLowerCase().includes(searchQuery.toLowerCase()) ||
      app.category.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-border/60 pb-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center text-indigo-400">
            <List className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-base font-semibold text-foreground flex items-center gap-2">
              Windows Application Catalog
            </h2>
            <p className="text-xs text-muted-foreground">
              Automatically discovered executables and Start Menu shortcuts ready for radial dial assignment
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={refreshApps}
            disabled={loading}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium border border-border bg-background hover:bg-muted text-foreground transition-all disabled:opacity-50"
            title="Rescan Windows Start Menu & Program Files"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`} />
            Rescan Applications
          </button>
        </div>
      </div>

      {/* Search and Filter Bar */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search applications by name, executable stem, or category..."
            className="w-full pl-9 pr-4 py-2 bg-background/80 border border-border rounded-xl text-xs text-foreground placeholder-muted-foreground focus:outline-none focus:ring-1 focus:ring-indigo-500"
          />
        </div>
        <span className="text-xs font-mono text-muted-foreground whitespace-nowrap">
          {filteredApps.length} of {apps.length} found
        </span>
      </div>

      {launchStatus && (
        <div className="p-3 rounded-lg bg-indigo-500/10 border border-indigo-500/30 text-indigo-400 text-xs flex items-center gap-2">
          <Check className="w-4 h-4" /> {launchStatus}
        </div>
      )}

      {/* Application Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 max-h-[480px] overflow-y-auto pr-1">
        {filteredApps.map((app) => (
          <div
            key={app.id}
            className="p-4 rounded-xl border border-border bg-background/50 hover:border-indigo-500/40 transition-all flex flex-col justify-between gap-3 group"
          >
            <div>
              <div className="flex items-start justify-between gap-2">
                <span className="font-semibold text-xs text-foreground truncate block" title={app.name}>
                  {app.name}
                </span>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-muted text-muted-foreground whitespace-nowrap">
                  {app.category}
                </span>
              </div>
              <p className="text-[11px] font-mono text-muted-foreground truncate mt-1" title={app.path}>
                {app.path}
              </p>
            </div>

            <div className="flex items-center justify-between gap-2 pt-2 border-t border-border/60">
              <button
                onClick={() => handleLaunch(app.path, app.name)}
                className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-indigo-600/10 hover:bg-indigo-600 text-indigo-400 hover:text-white text-xs font-medium transition-all"
              >
                <Play className="w-3 h-3" /> Launch
              </button>

              {/* Assign to Sector Dropdown */}
              <div className="flex items-center gap-1">
                <span className="text-[10px] text-muted-foreground">Assign:</span>
                <div className="flex items-center gap-0.5">
                  {[0, 1, 2, 3, 4, 5].map((sectorId) => {
                    const isAssigned = sectors && sectors[sectorId]?.target === app.path;
                    const justClicked = assignedAppId === `${app.id}-${sectorId}`;
                    return (
                      <button
                        key={sectorId}
                        onClick={() => handleAssign(sectorId, app)}
                        className={`w-6 h-6 rounded text-[10px] font-mono font-bold transition-all ${
                          justClicked
                            ? "bg-emerald-600 text-white"
                            : isAssigned
                            ? "bg-indigo-600 text-white"
                            : "bg-muted hover:bg-indigo-500/20 text-muted-foreground hover:text-foreground"
                        }`}
                        title={`Assign to Sector ${sectorId}`}
                      >
                        {sectorId}
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
          </div>
        ))}

        {filteredApps.length === 0 && !loading && (
          <div className="col-span-full py-12 text-center text-xs text-muted-foreground">
            No applications matched &quot;{searchQuery}&quot;. Try a different search term or rescan.
          </div>
        )}
      </div>
    </div>
  );
};
