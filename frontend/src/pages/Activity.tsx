import { Suspense, useState, useEffect } from "react";
import { useLocation } from "react-router-dom";
import { useActivityStats } from "../hooks/useActivityStats.ts";
import "../styles/activity.css";

interface TimePeriodTabsProps {
    timePeriods: string[];
    selectedPeriod: string; // Allow any dynamic time period
    setSelectedPeriod: (period: string) => void; // Update function
}

const TimePeriodTabs: React.FC<TimePeriodTabsProps> = ({
    timePeriods,
    selectedPeriod,
    setSelectedPeriod,
}) => {
    return (
        <div className="tab-container">
            {timePeriods.map((period) => (
                <span
                    key={period}
                    onClick={() => setSelectedPeriod(period)}
                    className={`activity-tab ${selectedPeriod === period ? "activity-tab-active" : ""}`}
                >
                    {period}
                </span>
            ))}
        </div>
    );
};

export default function Activity() {
    const { pathname } = useLocation(); // Get current URL path
    const { data, isLoading, error } = useActivityStats();

    type ActivityKeys = {
        activity_stat_names: Record<string, string>;
        time_windows: Record<string, string>;
        select_accounts_by: Record<string, string>;
    };

    // Activity stats keys
    const [statsNames, setActivityStatNames] = useState<string[]>([]);
    const [timeWindows, setTimeWindows] = useState<string[]>([]);
    const [selectAccountsBy, setSelectAccountsBy] = useState<
        { key: string; value: string }[]
    >([]);

    useEffect(() => {
        async function loadActivityKeys() {
            try {
                const response = await fetch("/activity_keys.json");
                if (!response.ok) {
                    throw new Error(
                        `Failed to load activity keys: ${response.statusText}`,
                    );
                }
                const keys: ActivityKeys = await response.json();

                setActivityStatNames(Object.values(keys.activity_stat_names));
                setTimeWindows(Object.values(keys.time_windows));
                setSelectAccountsBy(
                    Object.entries(keys.select_accounts_by).map(
                        ([key, value]) => ({ key, value }),
                    ),
                );
            } catch (err) {
                console.error(err);
            }
        }

        loadActivityKeys();
    }, []);

    const [statPeriods, setStatPeriods] = useState<Record<string, string>>(
        () => {
            const defaultStatPeriods: Record<string, string> = {};

            // Ensure we have both statsNames and timeWindows before initializing
            if (statsNames.length > 0 && timeWindows.length > 0) {
                statsNames.forEach((stat) => {
                    defaultStatPeriods[stat] = timeWindows[0]; // Default to first available time window
                });
            }

            return defaultStatPeriods;
        },
    );

    // Update `statPeriods` when `statsNames` or `timeWindows` change
    useEffect(() => {
        if (statsNames.length === 0 || timeWindows.length === 0) return;

        const defaultStatPeriods: Record<string, string> = {};
        statsNames.forEach((stat) => {
            defaultStatPeriods[stat] = timeWindows[0]; // Default to first available time period
        });

        setStatPeriods(defaultStatPeriods);
    }, [statsNames, timeWindows]);

    return (
        <div>
            {/* Activity Monitor Page */}
            {pathname === "/activity" && (
                <div>
                    {!data ||
                        (error && (
                            <p className="error-text">Error loading data</p>
                        ))}
                    <Suspense
                        fallback={<p className="loading-text">Loading...</p>}
                    >
                        {isLoading ? (
                            <p className="loading-text">Loading...</p>
                        ) : (
                            <div>
                                <div className="activity-cards">
                                    {statsNames.map((statName) => (
                                        <section
                                            key={statName}
                                            className="activity-section"
                                        >
                                            <span className="activity-title">
                                                {statName.toUpperCase()}
                                            </span>
                                            <TimePeriodTabs
                                                timePeriods={timeWindows}
                                                selectedPeriod={
                                                    statPeriods[statName]
                                                }
                                                setSelectedPeriod={(period) =>
                                                    setStatPeriods((prev) => ({
                                                        ...prev,
                                                        [statName]: period,
                                                    }))
                                                }
                                            />
                                            <div className="stat-value">
                                                {data?.stats[statName][
                                                    statPeriods[statName]
                                                ] ?? 0}
                                            </div>
                                        </section>
                                    ))}
                                </div>
                                <div className="activity-cards">
                                    {selectAccountsBy.map(({ key, value }) => {
                                        const accounts =
                                            data?.selected_accounts[value] ??
                                            [];
                                        return (
                                            <section
                                                key={value}
                                                className="accounts-section"
                                            >
                                                <span className="activity-title">
                                                    {value}
                                                </span>
                                                {accounts.length > 0 ? (
                                                    <ul className="account-list">
                                                        {accounts.map(
                                                            (
                                                                account,
                                                                index,
                                                            ) => (
                                                                <li
                                                                    key={index}
                                                                    className="account-item"
                                                                >
                                                                    <span className="account-address">
                                                                        {
                                                                            account.address
                                                                        }
                                                                    </span>
                                                                    <span className="account-detail">
                                                                        {key ===
                                                                        "ACCOUNTS__RECENT"
                                                                            ? `Deployed: ${new Date(account.creation_timestamp!).toLocaleString()}`
                                                                            : `Gas Used: ${Number(account.gas_used!).toLocaleString()}`}
                                                                    </span>
                                                                </li>
                                                            ),
                                                        )}
                                                    </ul>
                                                ) : (
                                                    <p className="no-accounts">
                                                        No accounts found.
                                                    </p>
                                                )}
                                            </section>
                                        );
                                    })}
                                </div>
                            </div>
                        )}
                    </Suspense>
                </div>
            )}
        </div>
    );
}
