import React, {
  SetStateAction,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import cx from "classnames";
import { Res, useFetch } from "use-http";

import shieldImg from "./img/shield.svg";
import { randInt } from "./utils";

interface AppState {
  overrideFocus: boolean;
  artificialLatency: number;
  queryLog: Array<LogItem>;
}

interface LogItem {
  url: URL;
  suggestions: Array<Suggestion> | null;
  wasSent: boolean;
  startTime: Date;
  endTime: Date | null;
}

interface MerinoApiResponse {
  suggestions: Array<Suggestion>;
}

interface Suggestion {
  title: string;
  url: string;
}

const App: React.FC = () => {
  const [state, setState] = useState<AppState>({
    overrideFocus: false,
    artificialLatency: 0,
    queryLog: [],
  });

  return (
    <div
      className="grid p-10 items-start"
      style={{ gridTemplateColumns: "1fr 2fr 1fr" }}
    >
      <Controls state={state} setState={setState} />
      <FakeQuantumBar state={state} setState={setState} />
      <Metrics state={state} setState={setState} />
    </div>
  );
};

interface StateProps {
  state: AppState;
  setState: React.Dispatch<SetStateAction<AppState>>;
}

const Controls: React.FC<StateProps> = ({ state, setState }) => {
  return (
    <div>
      <h1 className="font-bold">Controls</h1>
      <div>
        <input
          className="mr-1"
          type="checkbox"
          checked={state.overrideFocus}
          onChange={(ev) =>
            setState((s: AppState) => ({
              ...s,
              overrideFocus: ev.target.checked,
            }))
          }
        />
        <label>Stay expanded</label>
      </div>
      <div>
        <label className="block">
          Artificial Latency: {state.artificialLatency}ms
        </label>
        <input
          type="range"
          min={0}
          max={1000}
          step={10}
          value={state.artificialLatency}
          onChange={(ev) =>
            setState((prevState) => ({
              ...prevState,
              artificialLatency: parseInt(ev.target.value),
            }))
          }
        />
      </div>
    </div>
  );
};

const FakeQuantumBar: React.FC<StateProps> = ({ state, setState }) => {
  let [text, setText] = useState(window.location.toString());
  let [naturalFocus, setNaturalFocus] = useState(false);
  let inputEl = useRef<HTMLInputElement>(null);

  let focused = state.overrideFocus || naturalFocus;
  const expandSize = 16;
  const maxRequestLog = 25;

  const handleRecordRequest = useCallback(
    (partialInfo: { url: string; wasSent: boolean }) => {
      const logItem: LogItem = {
        url: new URL(partialInfo.url),
        wasSent: partialInfo.wasSent,
        startTime: new Date(),
        endTime: null,
        suggestions: null,
      };
      setState((prevState) => {
        let nextState = {
          ...prevState,
          queryLog: [...prevState.queryLog, logItem],
        };
        if (nextState.queryLog.length > maxRequestLog) {
          nextState.queryLog = nextState.queryLog.slice(-maxRequestLog);
        }
        return nextState;
      });
    },
    [setState]
  );

  const handleRecordResponse = useCallback(
    (response: Res<MerinoApiResponse>) => {
      setState((prevState) => {
        // make a copy of the state, especially cloning the log items
        let nextState = {
          ...prevState,
          queryLog: prevState.queryLog.map((item) => ({ ...item })),
        };
        nextState.queryLog
          .filter(
            (item) =>
              item.url.toString() === response.url &&
              item.wasSent &&
              item.endTime === null
          )
          .forEach((item) => {
            item.endTime = new Date();
            item.suggestions = response.data?.suggestions ?? null;
          });
        return nextState;
      });
    },
    [setState]
  );

  return (
    <div className="max-w-3xl w-full relative mx-auto">
      {/* background white layer */}
      <div
        className={
          "bg-white flex flex-row border rounded absolute z-0 border-content transition-all"
        }
        style={{
          borderColor: "rgba(205, 199, 194)",
          width: focused ? `calc(48rem + ${expandSize}px)` : "48rem",
          height: focused ? `calc(100% + ${expandSize}px)` : "32px",
          marginTop: focused ? -expandSize / 2 : 0,
          marginLeft: focused ? -expandSize / 2 : 0,
        }}
      />
      {/* input area */}
      <div className="z-10 flex flex-row relative items-center space-x-2 px-2 py-1">
        <img src={shieldImg} />
        <input
          className="focus:outline-none w-full bg-transparent"
          ref={inputEl}
          value={text}
          onChange={(ev) => setText(ev.target.value)}
          onFocus={(ev) => {
            setNaturalFocus(true);
            inputEl.current?.setSelectionRange(0, text.length);
          }}
          onBlur={() => {
            setNaturalFocus(false);
            inputEl.current?.setSelectionRange(0, 0);
            if (inputEl.current?.value.trim() === "") {
              setText(window.location.toString());
            }
          }}
        />
      </div>
      {/* suggestions */}
      {focused ? (
        <CompletionList
          className="z-10 relative"
          query={text}
          recordRequest={handleRecordRequest}
          recordResponse={handleRecordResponse}
          artificialLatency={state.artificialLatency}
        />
      ) : null}
    </div>
  );
};

const Metrics: React.FC<StateProps> = ({ state, setState }) => {
  return (
    <div>
      <h1 className="font-bold">Queries</h1>
      <div className="flex flex-col-reverse space-y-1 text-sm">
        {state.queryLog.map((logItem, idx) => {
          const query = logItem.url.searchParams.get("q");
          return (
            <div key={idx} className="flex flex-row space-x-2">
              <span className="flex-grow">{query ?? "<null>"}</span>
              {logItem.endTime && (
                <span>{+logItem.endTime - +logItem.startTime}ms</span>
              )}
              {logItem.wasSent ? (
                <span title="Request was sent">✔</span>
              ) : (
                <span title="Request was blocked">✖</span>
              )}
              {logItem.suggestions ? (
                <code title={`${logItem.suggestions.length} suggestions`}>
                  [{logItem.suggestions.length}]
                </code>
              ) : (
                <code title={logItem.endTime ? "No results" : "Loading"}>
                  [-]
                </code>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

interface CompletionListProps {
  className: string;
  query: string;
  recordRequest: (info: { url: string; wasSent: boolean }) => void;
  recordResponse: (response: Res<MerinoApiResponse>) => void;
  artificialLatency: number;
}

function shouldSendQuery(query: string): boolean {
  if (query.length < 3) {
    return false;
  }
  if (query.includes(" ")) {
    return true;
  }
  if (query.match(/htt(ps?:)?/i)) {
    return false;
  }
  if (query.includes(".") && !query.includes(" ")) {
    return false;
  }
  return true;
}

const CompletionList: React.FC<CompletionListProps> = ({
  className,
  query,
  recordRequest,
  recordResponse,
  artificialLatency,
}) => {
  const placeholderCompletions = useMemo(() => {
    const count = randInt(3, 8);
    let completions = [];
    while (completions.length < count) {
      completions.push(<CompletionPlaceHolder key={completions.length} />);
    }
    return completions;
  }, [query]);

  const {
    loading,
    error,
    data,
  }: {
    loading: boolean;
    error?: Error;
    data?: MerinoApiResponse;
  } = useFetch(
    `http://localhost:8080/api/v1/suggest?q=${query}`,
    {
      interceptors: {
        request: async ({ options, url }) => {
          if (url) {
            let parsedUrl = new URL(url);
            let query = parsedUrl.searchParams.get("q");
            if (query && shouldSendQuery(query)) {
              recordRequest({ url, wasSent: true });
            } else {
              recordRequest({ url, wasSent: false });
              return {};
            }
          }
          if (artificialLatency > 0) {
            await new Promise((resolve) =>
              setTimeout(resolve, artificialLatency)
            );
          }
          return options;
        },
        response: async ({ response }) => {
          recordResponse(response);
          return response;
        },
      },
    },
    [query]
  );

  let completions = [...placeholderCompletions];
  for (let suggestion of data?.suggestions ?? []) {
    completions.push(
      <CompletionItem
        key={suggestion.url}
        title={suggestion.title}
        url={suggestion.url}
        icon={
          <div
            className={`inline-block w-4 h-4 bg-gray-500 flex-grow-0 flex-shrink-0`}
          ></div>
        }
      />
    );
  }

  return (
    <div
      className={cx("border-t mt-1", className)}
      style={{
        top: -1,
        borderColor: "rgba(205, 199, 194)",
      }}
    >
      {completions}
    </div>
  );
};

interface CompletionItemProps {
  icon: React.ReactElement;
  title: string;
  url: string;
}

const CompletionItem: React.FC<CompletionItemProps> = ({
  icon,
  title,
  url,
}) => {
  return (
    <div className="mx-2 mt-2 flex flex-row items-center space-x-2">
      <div className="w-4 h-4">{icon}</div>
      <div className="flex-shrink">{title}</div>
      <div>—</div>
      <div className="flex-shrink text-xs text-opacity-50">{url}</div>
    </div>
  );
};

const colors = ["red", "yellow", "green", "blue", "indigo", "purple", "pink"];

const CompletionPlaceHolder: React.FC = () => {
  const titleWidth = useMemo(() => randInt(75, 300), []);
  const urlWidth = useMemo(() => randInt(100, 400), []);
  const color = useMemo(() => {
    let idx = Math.floor(Math.random() * colors.length);
    return colors[idx];
  }, [colors]);

  return (
    <div className="mx-2 mt-2 flex flex-row items-center space-x-2">
      <div
        className={`inline-block w-4 h-4 bg-${color}-500 flex-grow-0 flex-shrink-0`}
      ></div>
      <div
        className="h-4 bg-gray-300 flex-shrink"
        style={{ width: titleWidth }}
      ></div>
      <div>—</div>
      <div
        className="h-2.5 bg-blue-400 flex-shrink"
        style={{ width: urlWidth }}
      ></div>
    </div>
  );
};

export default App;
