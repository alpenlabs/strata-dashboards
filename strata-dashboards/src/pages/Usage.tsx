import { useState, useEffect } from "react";
import { useLocation } from "react-router-dom";
import { useUsageStats } from "../hooks/useUsageStats.ts";
  
interface TimePeriodTabsProps {
    timePeriods: string[];
    selectedPeriod: string; // Allow any dynamic time period
    setSelectedPeriod: (period: string) => void; // Update function
}

const TimePeriodTabs: React.FC<TimePeriodTabsProps> = ({ timePeriods, selectedPeriod, setSelectedPeriod }) => {
    return (
      <div className="tab-container">
        {timePeriods.map((period) => (
          <span
            key={period}
            onClick={() => setSelectedPeriod(period)}
            className={`usage-tab ${selectedPeriod === period ? "usage-tab-active" : ""}`}
          >
            {period}
          </span>
        ))}
      </div>
    );
};

export default function Usage() {
    const { pathname } = useLocation(); // Get current URL path
    const { data, isLoading, error } = useUsageStats();
    const aggrStats = ["User ops", "Gas used", "Unique active accounts"];
    const timePeriods = ["24h", "30d", "YTD"];
    if (isLoading) return <p className="loading-text">Loading...</p>
    if (error || !data) return <p className="error-text">Error loading data</p>

    console.log(data);
    const [statPeriods, setStatPeriods] = useState<Record<string, string>>({});
    useEffect(() => {
        const defaultStatPeriods: Record<string, string> = {};
        aggrStats.forEach((stat) => {
            defaultStatPeriods[stat] = timePeriods[0]; // Default to first available time period
        });

        setStatPeriods(defaultStatPeriods);
    }, [data]); // Ensures this only runs when `data` updates

    // Sort recent accounts by timestamp and top gas consumers by gas used
    const recent_accounts = data.sel_accounts["Recent accounts"]["recent"].sort((a, b) => 
        Number(b.deployed_at) - Number(a.deployed_at));
    const top_gas_consumers = data.sel_accounts["Top gas consumers"]["24h"].sort((a, b) => 
        Number(b.gas_used) - Number(a.gas_used));

    return (
        <div className="usage-content">
            {/* Usage Monitor Page */}
            {pathname === "/usage" && (
                <div className="usage-container">
                    <div className="usage-cards">
                        {aggrStats.map((statName) => (
                            <section key={statName} className="usage-section">
                                <text className="usage-title">{statName}</text> {/* Format title */}
                                <TimePeriodTabs
                                    timePeriods={timePeriods}
                                    selectedPeriod={statPeriods[statName]}
                                    setSelectedPeriod={(period) =>
                                        setStatPeriods((prev) => ({ ...prev, [statName]: period }))
                                    }
                                />
                                <div className="stat-value">
                                    {data.stats[statName][statPeriods[statName]]}
                                </div>
                            </section>
                        ))}
                    </div>
                    <div className="usage-cards">
                        <section key="Recent accounts" className="accounts-section">
                            <text className="usage-title">Recent accounts</text> {/* Format title */}
                            {recent_accounts.length > 0 ? (
                                <ul className="account-list">
                                {data.sel_accounts["Recent accounts"]["recent"].map((account, index) => (
                                    <li key={index} className="account-item">
                                        <span className="account-address">{account.address}</span>
                                        <span className="account-detail">
                                            Deployed: {new Date(account.deployed_at).toLocaleString()}
                                        </span>
                                    </li>
                                ))}
                                </ul>
                            ) : (
                                <p className="no-accounts">No accounts found.</p>
                            )}
                        </section>
                        <section key="Top gas consumers" className="accounts-section">
                            <text className="usage-title">Top gas consumers (24h)</text> {/* Format title */}
                            {top_gas_consumers.length > 0 ? (
                                <ul className="account-list">
                                {top_gas_consumers.map((account, index) => (
                                    <li key={index} className="account-item">
                                        <span className="account-address">{account.address}</span>
                                        <span className="account-detail">
                                            Gas Used: {Number(account.gas_used).toLocaleString()}
                                        </span>
                                    </li>
                                ))}
                                </ul>
                            ) : (
                                <p className="no-accounts">No accounts found.</p>
                            )}                                    
                        </section>
                    </div>
                </div>
            )}
        </div>
    );
}