import { createRequire } from 'node:module';
import { randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import type {
  QueryRequest,
  QueryResponse,
  MaintenanceRequest,
  ErrorBody,
  TopicContext,
  Capabilities,
} from './generated/contract.js';

export interface OpenOptions {
  configPath: string;
}
export interface RequestOptions {
  signal?: AbortSignal;
  requestId?: string;
}
type ParametersFor<O> = Omit<Extract<QueryRequest, { operation: O }>, 'operation'>;
type MaintenanceParameters<O> = Omit<Extract<MaintenanceRequest, { operation: O }>, 'operation'>;

export type { Capabilities } from './generated/contract.js';

export class ProjectKnowledgeError extends Error {
  readonly code: string;
  readonly retryable: boolean;
  readonly issues: NonNullable<ErrorBody['issues']>;
  readonly runId: string | undefined;
  readonly phase: string | undefined;

  constructor(body: ErrorBody) {
    super(body.message);
    this.name = 'ProjectKnowledgeError';
    this.code = body.code;
    this.retryable = body.retryable;
    this.issues = body.issues ?? [];
    this.runId = body.runId ?? undefined;
    this.phase = body.phase ?? undefined;
  }
}

function failure(code: string, message: string): ProjectKnowledgeError {
  return new ProjectKnowledgeError({ code, message, retryable: false, issues: [] });
}

function asError(error: unknown): ProjectKnowledgeError {
  if (error instanceof ProjectKnowledgeError) return error;
  if (error instanceof Error) {
    try {
      const body = JSON.parse(error.message) as { error?: ErrorBody };
      if (body.error?.code && typeof body.error.message === 'string') {
        return new ProjectKnowledgeError(body.error);
      }
    } catch {
      /* Transport failures do not expose native or provider internals. */
    }
  }
  return failure('INTERNAL_ERROR', 'Native operation failed');
}

interface NativeClient {
  execute(id: string, raw: string, maintenance: boolean): Promise<string>;
  cancel(id: string): boolean;
  close(): Promise<void>;
}
interface NativeModule {
  NativeClient: { open(configPath: string, maintenance: boolean): Promise<NativeClient> };
  getCapabilities(): Promise<string>;
  validateRequest(raw: string, maintenance: boolean): Promise<void>;
}
let loaded: NativeModule | undefined;
function native(): NativeModule {
  if (loaded) return loaded;
  try {
    const candidate = createRequire(import.meta.url)('@project-knowledge/native') as NativeModule;
    if (
      typeof candidate.NativeClient?.open !== 'function' ||
      typeof candidate.getCapabilities !== 'function' ||
      typeof candidate.validateRequest !== 'function'
    ) {
      throw new Error('incompatible native module');
    }
    loaded = candidate;
    return candidate;
  } catch {
    throw failure(
      'RUNTIME_MISSING',
      `Install @project-knowledge/native@0.1.0 and its matching platform package for ${process.platform}/${process.arch} (Node 24+).`,
    );
  }
}

/** Query connections are read only; maintenance requires openMaintenance(). */
export class ProjectKnowledge {
  #handle: NativeClient;
  #closed = false;
  #closing: Promise<void> | undefined;
  #pending = new Set<string>();

  protected constructor(handle: NativeClient) {
    this.#handle = handle;
  }

  static async open(options: OpenOptions): Promise<ProjectKnowledge> {
    try {
      return new ProjectKnowledge(
        await native().NativeClient.open(resolve(options.configPath), false),
      );
    } catch (error) {
      throw asError(error);
    }
  }

  static async openMaintenance(options: OpenOptions): Promise<MaintenanceClient> {
    try {
      return new MaintenanceClient(
        await native().NativeClient.open(resolve(options.configPath), true),
      );
    } catch (error) {
      throw asError(error);
    }
  }

  static async getCapabilities(): Promise<Capabilities> {
    try {
      return JSON.parse(await native().getCapabilities()) as Capabilities;
    } catch (error) {
      throw asError(error);
    }
  }

  /** Core schema validation without filesystem or database operations. */
  static async validateRequest(request: unknown, maintenance = false): Promise<void> {
    try {
      await native().validateRequest(JSON.stringify(request) ?? 'null', maintenance);
    } catch (error) {
      throw asError(error);
    }
  }

  async getCapabilities(): Promise<Capabilities> {
    if (this.#closed) throw failure('CLOSED', 'Client is closed');
    return ProjectKnowledge.getCapabilities();
  }

  protected async dispatch<T>(
    request: QueryRequest | MaintenanceRequest,
    maintenance: boolean,
    options: RequestOptions = {},
  ): Promise<T> {
    if (this.#closed) throw failure('CLOSED', 'Client is closed');
    if (options.signal?.aborted) throw failure('CANCELLED', 'Request cancelled');
    const id = options.requestId ?? randomUUID();
    if (this.#pending.has(id)) throw failure('INVALID_REQUEST', 'Request ID is already in use');
    this.#pending.add(id);
    let abort: (() => void) | undefined;
    try {
      let raw: string;
      try {
        raw = JSON.stringify(request) ?? 'null';
      } catch {
        throw failure('INVALID_REQUEST', 'Request must be JSON serializable');
      }
      // Native registers the ID synchronously, before its background task is scheduled.
      const response = this.#handle.execute(id, raw, maintenance);
      abort = () => {
        this.#handle.cancel(id);
      };
      options.signal?.addEventListener('abort', abort, { once: true });
      if (options.signal?.aborted) abort();
      return JSON.parse(await response) as T;
    } catch (error) {
      throw asError(error);
    } finally {
      this.#pending.delete(id);
      if (abort) options.signal?.removeEventListener('abort', abort);
    }
  }

  execute(request: QueryRequest, options?: RequestOptions): Promise<QueryResponse> {
    return this.dispatch(request, false, options);
  }
  status(request: ParametersFor<'status'> = {}, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'status' }, options);
  }
  search(request: ParametersFor<'search'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'search' }, options);
  }
  locateFeature(request: ParametersFor<'locateFeature'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'locateFeature' }, options);
  }
  trace(request: ParametersFor<'trace'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'trace' }, options);
  }
  neighbors(request: ParametersFor<'neighbors'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'neighbors' }, options);
  }
  getCodeMap(request: ParametersFor<'codeMap'> = {}, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'codeMap' }, options);
  }
  readSource(request: ParametersFor<'readSource'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'readSource' }, options);
  }
  locatePage(request: ParametersFor<'page'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'page' }, options);
  }
  explainWorkflow(request: ParametersFor<'workflow'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'workflow' }, options);
  }
  explainReport(request: ParametersFor<'report'>, options?: RequestOptions) {
    return this.execute({ ...request, operation: 'report' }, options);
  }

  cancel(requestId: string): boolean {
    return this.#handle.cancel(requestId);
  }

  close(): Promise<void> {
    if (!this.#closing) {
      this.#closed = true;
      try {
        this.#closing = this.#handle.close().catch((error: unknown) => {
          throw asError(error);
        });
      } catch (error) {
        this.#closing = Promise.reject(asError(error));
      }
    }
    return this.#closing;
  }

  async [Symbol.asyncDispose](): Promise<void> {
    await this.close();
  }
}

export class MaintenanceClient extends ProjectKnowledge {
  constructor(handle: NativeClient) {
    super(handle);
  }
  maintain(request: MaintenanceRequest, options?: RequestOptions): Promise<unknown> {
    return this.dispatch(request, true, options);
  }
  index(request: MaintenanceParameters<'index'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'index' }, options);
  }
  discoverPlugins(options?: RequestOptions) {
    return this.maintain({ operation: 'discoverPlugins' }, options);
  }
  submitPlugin(request: MaintenanceParameters<'submitPlugin'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'submitPlugin' }, options);
  }
  preparePlugin(request: MaintenanceParameters<'preparePlugin'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'preparePlugin' }, options);
  }
  validatePlugin(request: MaintenanceParameters<'validatePlugin'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'validatePlugin' }, options);
  }
  activatePlugin(request: MaintenanceParameters<'activatePlugin'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'activatePlugin' }, options);
  }
  rollbackPlugin(request: MaintenanceParameters<'rollbackPlugin'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'rollbackPlugin' }, options);
  }
  collect(options?: RequestOptions) {
    return this.maintain({ operation: 'collect' }, options);
  }
  exportContext(
    request: MaintenanceParameters<'exportContext'>,
    options?: RequestOptions,
  ): Promise<TopicContext> {
    return this.dispatch({ ...request, operation: 'exportContext' }, true, options);
  }
  importTopic(request: MaintenanceParameters<'importTopic'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'importTopic' }, options);
  }
  importConventions(request: MaintenanceParameters<'importConventions'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'importConventions' }, options);
  }
  generateSkill(request: MaintenanceParameters<'generateSkill'>, options?: RequestOptions) {
    return this.maintain({ ...request, operation: 'generateSkill' }, options);
  }
}
